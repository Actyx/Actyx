use super::{
    non_empty::{NoElements, NonEmptyString},
    parse_utils::P,
    parser::Rule,
};
use crate::{
    language::{
        parse_utils::{Ext, Spanned},
        parser::{r_bool, r_nonempty_string, r_string, NoVal},
    },
    Ident, NonEmptyVec,
};
use once_cell::sync::Lazy;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum Type {
    /// The bottom type | the impossible type. equivalent to union of no types.
    /// When this type arises, evaluation must not happen i.e. AQL execution must stop at type-checking
    Never(Vec<String>),
    /// Indicates that there is a lack of value
    NoValue,
    Atom(TypeAtom),
    Union(Arc<(Type, Type)>),
    Intersection(Arc<(Type, Type)>),
    Array(Arc<Type>),
    Dict(Arc<Type>),
    Tuple(NonEmptyVec<Type>),
    Record(NonEmptyVec<(Label, Type)>),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Hierarchy {
    Disjointed,
    Equal,
    Supertype,
    Subtype,
}

impl Type {
    pub(crate) fn never(
        iter: impl IntoIterator<Item = Type, IntoIter = impl DoubleEndedIterator<Item = Type>>,
    ) -> Type {
        Type::Never(
            iter.into_iter()
                .filter_map(|x| match x {
                    Type::Never(x) => Some(x),
                    _ => None,
                })
                .flatten()
                .collect(),
        )
    }

    /// Build a union out of all types in the iterator argument
    ///
    /// # Panics
    ///
    /// Panics if the iterator does not yield any value!
    pub(crate) fn union(
        iter: impl IntoIterator<Item = Type, IntoIter = impl DoubleEndedIterator<Item = Type>>,
    ) -> Type {
        let rebuilt = iter.into_iter().rev().fold(None, |acc, next| match acc {
            None => Some(next),
            Some(prev) => Some(Type::Union(Arc::new((prev, next)))),
        });

        rebuilt.unwrap_or(Type::Never(vec!["empty union".to_string()]))
    }

    /// Build an intersection out of all types in the iterator argument
    ///
    /// # Panics
    ///
    /// Panics if the iterator does not yield any value!
    pub(crate) fn intersection(
        iter: impl IntoIterator<Item = Type, IntoIter = impl DoubleEndedIterator<Item = Type>>,
    ) -> Type {
        let rebuilt = iter.into_iter().rev().fold(None, |acc, next| match acc {
            None => Some(next),
            Some(prev) => Some(Type::Intersection(Arc::new((prev, next)))),
        });

        rebuilt.unwrap_or(Type::Never(vec!["empty intersection".to_string()]))
    }

    /// Attempt to apply collapse and sorts to intersections, unions, records
    pub(crate) fn collapse(self) -> Type {
        match self {
            Type::Intersection(intersection) => {
                let (a, b) = intersection.as_ref();
                let (a, b) = (a.clone().collapse(), b.clone().collapse());

                if a == b {
                    return a.clone();
                }

                fn distribute_and_unify<'a>(
                    set_a: impl IntoIterator<Item = &'a Type>,
                    set_b: impl IntoIterator<Item = &'a Type>,
                ) -> Type {
                    let set_a = set_a
                        .into_iter()
                        .cloned()
                        .map(|x: Type| x.collapse())
                        .collect::<Vec<Type>>();

                    let set_b = set_b
                        .into_iter()
                        .cloned()
                        .map(|x: Type| x.collapse())
                        .collect::<Vec<Type>>();

                    let permutations = set_a.iter().flat_map(|a| {
                        set_b
                            .iter()
                            .map(|b| Type::intersection([a.clone(), b.clone()]).collapse())
                    });

                    Type::union(permutations).collapse()
                }

                match (&a, &b) {
                    // there should not be intersections of intersections after a and b is collapsed
                    (Type::Intersection(_), _) | (_, Type::Intersection(_)) => {
                        Type::Never(vec!["intersection could not exist after a collapse".into()])
                    }
                    (Type::Union(union_a), Type::Union(union_b)) => {
                        let (a1, a2) = union_a.as_ref();
                        let (b1, b2) = union_b.as_ref();
                        distribute_and_unify([a1, a2], [b1, b2])
                    }
                    (Type::Union(union), x) => {
                        let (a, b) = union.as_ref();
                        distribute_and_unify([a, b], [x])
                    }
                    (x, Type::Union(union)) => {
                        let (a, b) = union.as_ref();
                        distribute_and_unify([a, b], [x])
                    }
                    // intersection of records
                    (Type::Record(a), Type::Record(b)) => {
                        let mut fields = BTreeMap::<&Label, Type>::new();

                        a.iter().chain(b.iter()).for_each(|(label, ty)| {
                            match fields.get(&label) {
                                None => fields.insert(label, ty.clone().collapse().clone()),
                                Some(old) => {
                                    fields.insert(label, Type::intersection([ty.clone(), old.clone()]).collapse())
                                }
                            };
                        });

                        Type::Record(
                            fields
                                .into_iter()
                                .map(|(label, ty)| (label.clone(), ty.clone()))
                                .collect::<Vec<(Label, Type)>>()
                                .try_into()
                                .unwrap(),
                        )
                        .collapse()
                    }
                    (Type::Dict(a), Type::Dict(b)) => Type::Dict(
                        Type::intersection([a.as_ref().clone(), b.as_ref().clone()])
                            .collapse()
                            .into(),
                    ),
                    (Type::Array(a), Type::Array(b)) => Type::Array(
                        Type::intersection([a.as_ref().clone(), b.as_ref().clone()])
                            .collapse()
                            .into(),
                    ),
                    _ => match &a.hierarchy_towards(&b) {
                        Hierarchy::Equal => a,
                        Hierarchy::Supertype => b,
                        Hierarchy::Subtype => a,
                        Hierarchy::Disjointed => {
                            Type::Never(vec![format!("{:?} and {:?} cannot be intersected", a, b)])
                        }
                    },
                }
            }
            Type::Union(x) => {
                let mut collapsing = vec![];

                Type::spread_union_collapsing(x.as_ref())
                    .into_iter()
                    .for_each(|new| match collapsing.last_mut() {
                        None => collapsing.push(new),
                        Some(last) => {
                            match (&new, &last) {
                                // Array and Dictionaries are not merged
                                (Type::Array(_), Type::Array(_)) => collapsing.push(new),
                                (Type::Dict(_), Type::Dict(_)) => collapsing.push(new),
                                _ => match new.hierarchy_towards(last) {
                                    Hierarchy::Disjointed => collapsing.push(new), // push
                                    Hierarchy::Supertype => *last = new,           // replace last with supertype
                                    Hierarchy::Subtype | Hierarchy::Equal => {}    // do nothing
                                },
                            }
                        }
                    });

                Type::union(collapsing)
            }
            Type::Record(fields) => {
                let fields = fields
                    .iter()
                    .cloned()
                    .map(|(label, ty)| (label, ty.collapse()))
                    .collect::<BTreeMap<_, _>>(); // collecting into btreemap sorts the fields

                let never_fields = fields
                    .iter()
                    .filter_map(|(_label, ty)| match ty {
                        Type::Never(_) => Some(ty.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                if !never_fields.is_empty() {
                    Type::never(never_fields)
                } else {
                    Type::Record(fields.into_iter().collect::<Vec<_>>().try_into().unwrap())
                }
            }
            Type::Never(_) | Type::NoValue | Type::Atom(_) => self,
            Type::Array(x) => Type::Array(x.as_ref().clone().collapse().into()),
            Type::Dict(x) => Type::Dict(x.as_ref().clone().collapse().into()),
            Type::Tuple(tuple) => Type::Tuple(
                tuple
                    .iter()
                    .map(|x| x.clone().collapse())
                    .collect::<Vec<_>>()
                    .try_into()
                    .expect("should return non-empty vec"),
            ),
        }
    }

    /// Determine the hierarchy of a type toward another
    /// # Caveat
    /// Collapse must be done before calculating the hierarchy, especially on
    /// intersections or aggregates that contains intersections.
    ///
    /// Hierarchy of intersections will always return `Hierarchy::Disjointed`
    /// because this function must not produce new type.
    ///
    /// Collapsing may occasionally accelerate calculation e.g. equal records wil instantly be detected without any complex calculations
    fn hierarchy_towards(&self, other: &Type) -> Hierarchy {
        if self == other {
            return Hierarchy::Equal;
        }

        fn compare_unions(a: &BTreeSet<&Type>, b: &BTreeSet<&Type>) -> Hierarchy {
            let mut a_has_supertype = BTreeSet::<&Type>::new();
            let mut b_has_supertype = BTreeSet::<&Type>::new();
            let mut registered_equal = BTreeSet::<&Type>::new();

            a.iter().for_each(|a_member| {
                b.iter().for_each(|b_member| {
                    match a_member.hierarchy_towards(b_member) {
                        Hierarchy::Supertype => b_has_supertype.insert(b_member),
                        Hierarchy::Subtype => a_has_supertype.insert(a_member),
                        Hierarchy::Equal => registered_equal.insert(a_member),
                        Hierarchy::Disjointed => false,
                    };
                })
            });

            if registered_equal.len() == a.len() && registered_equal.len() == b.len() {
                Hierarchy::Equal
            } else if a_has_supertype.len() == a.len() && b_has_supertype.is_empty() {
                Hierarchy::Subtype
            } else if b_has_supertype.len() == b.len() && a_has_supertype.is_empty() {
                Hierarchy::Supertype
            } else {
                Hierarchy::Disjointed
            }
        }

        match (self, other) {
            (_, Type::Never(_)) => Hierarchy::Supertype,
            (Type::Never(_), _) => Hierarchy::Subtype,
            (Type::Atom(TypeAtom::Universal), Type::Atom(TypeAtom::Universal)) => Hierarchy::Equal,
            (Type::Atom(TypeAtom::Universal), _) => Hierarchy::Supertype,
            (_, Type::Atom(TypeAtom::Universal)) => Hierarchy::Subtype,
            (Type::Atom(a), Type::Atom(b)) => match (a, b) {
                // supertypes
                (TypeAtom::Bool(None), TypeAtom::Bool(Some(_))) => Hierarchy::Supertype,
                (TypeAtom::Number(None), TypeAtom::Number(Some(_))) => Hierarchy::Supertype,
                (TypeAtom::String(None), TypeAtom::String(Some(_))) => Hierarchy::Supertype,
                // subtypes
                (TypeAtom::Bool(Some(_)), TypeAtom::Bool(None)) => Hierarchy::Subtype,
                (TypeAtom::Number(Some(_)), TypeAtom::Number(None)) => Hierarchy::Subtype,
                (TypeAtom::String(Some(_)), TypeAtom::String(None)) => Hierarchy::Subtype,
                _ => Hierarchy::Disjointed,
            },
            (Type::Record(a), Type::Record(b)) => {
                let same_field_relationships = a
                    .iter()
                    .flat_map(|a| {
                        b.iter().filter_map(|b| match a.0 == b.0 {
                            true => Some(a.1.hierarchy_towards(&b.1)),
                            false => None,
                        })
                    })
                    .collect::<BTreeSet<Hierarchy>>();
                let a = a.iter().map(|(label, _)| label).collect::<BTreeSet<_>>();
                let b = b.iter().map(|(label, _)| label).collect::<BTreeSet<_>>();

                let same_fields = BTreeSet::intersection(&a, &b).cloned().collect::<BTreeSet<_>>();

                let has_disjointed = same_field_relationships.contains(&Hierarchy::Disjointed);
                let has_subtype = same_field_relationships.contains(&Hierarchy::Subtype);
                let has_supertype = same_field_relationships.contains(&Hierarchy::Supertype);

                if same_fields == b && same_fields == a && !has_disjointed && !has_supertype && !has_subtype {
                    Hierarchy::Equal
                } else if same_fields == a && !has_disjointed && !has_subtype {
                    Hierarchy::Supertype
                } else if same_fields == b && !has_disjointed && !has_supertype {
                    Hierarchy::Subtype
                } else {
                    Hierarchy::Disjointed
                }
            }
            (Type::Union(a), Type::Union(b)) => {
                compare_unions(&Type::spread_union(a.as_ref()), &Type::spread_union(b.as_ref()))
            }
            (Type::Union(a), b) => compare_unions(&Type::spread_union(a.as_ref()), &BTreeSet::from([b])),
            (a, Type::Union(b)) => compare_unions(&BTreeSet::from([a]), &Type::spread_union(b.as_ref())),
            (Type::Array(a), Type::Array(b)) => a.hierarchy_towards(b),
            (Type::Dict(a), Type::Dict(b)) => a.hierarchy_towards(b),
            // We don't that here! Intersection comparison needs collapsing and this function should be as cheap as possible
            // (Type::Intersection, _) | (_, Type::Intersection) => {},
            _ => Hierarchy::Disjointed,
        }
    }

    /// Spread union tree into a set of type references without collapsing
    fn spread_union((a, b): &(Type, Type)) -> BTreeSet<&Type> {
        let spread = spread(vec![a, b], |x| match x {
            Type::Union(union) => {
                let (a, b) = union.as_ref();
                Spread::Many(vec![a, b])
            }
            x => Spread::One(x),
        });

        spread.into_iter().collect()
    }

    /// Spread union tree into a set of types with collapsing
    fn spread_union_collapsing((a, b): &(Type, Type)) -> BTreeSet<Type> {
        let spread = spread(vec![a.clone(), b.clone()], |x| match x.clone().collapse() {
            Type::Union(union) => {
                let (a, b) = union.as_ref();
                Spread::Many(vec![a.clone(), b.clone()])
            }
            x => Spread::One(x),
        });

        spread.into_iter().collect()
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum TypeAtom {
    Null,
    Bool(Option<bool>),
    Number(Option<u64>),
    Timestamp,
    String(Option<String>),
    Universal,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum Label {
    String(NonEmptyString),
    Number(u64),
}

impl From<Ident> for Label {
    fn from(value: Ident) -> Self {
        Label::String(value.0)
    }
}

impl TryFrom<&str> for Label {
    type Error = NoElements;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self::String(NonEmptyString::try_from(value)?))
    }
}

pub fn r_type(p: P) -> anyhow::Result<Type> {
    static PRATT: Lazy<PrattParser<Rule>> = Lazy::new(|| {
        PrattParser::new()
            .op(Op::infix(Rule::type_union, Assoc::Left))
            .op(Op::infix(Rule::type_intersection, Assoc::Left))
            .op(Op::postfix(Rule::type_array))
            .op(Op::postfix(Rule::type_dict))
    });
    PRATT
        .map_primary(|mut p| {
            Ok(match p.as_rule() {
                Rule::type_null => Type::Atom(TypeAtom::Null),
                Rule::type_bool => Type::Atom(TypeAtom::Bool(None)),
                Rule::bool => Type::Atom(TypeAtom::Bool(Some(r_bool(p)))),
                Rule::type_number => Type::Atom(TypeAtom::Number(None)),
                Rule::natural => Type::Atom(TypeAtom::Number(Some(p.natural()?))),
                Rule::type_timestamp => Type::Atom(TypeAtom::Timestamp),
                Rule::type_string => Type::Atom(TypeAtom::String(None)),
                Rule::string => Type::Atom(TypeAtom::String(Some(r_string(p)?))),
                Rule::type_tuple => {
                    let span = p.as_span();
                    let ts = p
                        .inner()?
                        .filter(|p| p.as_rule() == Rule::r#type)
                        .map(r_type)
                        .collect::<Result<Vec<_>, _>>()?;
                    Type::Tuple(NonEmptyVec::try_from(ts).spanned(span)?)
                }
                Rule::type_record => {
                    let mut ts = vec![];
                    let span = p.as_span();
                    let mut p = p
                        .inner()?
                        .filter(|p| !matches!(p.as_rule(), Rule::curlyl | Rule::curlyr | Rule::comma | Rule::colon));
                    while let Some(mut label) = p.next() {
                        let label = match label.as_rule() {
                            Rule::nonempty_string => Label::String(r_nonempty_string(label)?),
                            Rule::natural => Label::Number(label.natural()?),
                            Rule::ident => Label::String(label.non_empty_string()?),
                            _ => unexpected!(label),
                        };
                        let t = r_type(p.next().ok_or(NoVal("value"))?)?;
                        ts.push((label, t));
                    }
                    Type::Record(NonEmptyVec::try_from(ts).spanned(span)?)
                }
                Rule::type_paren => r_type(p.into_inner().nth(1).ok_or(NoVal("nested type"))?)?,
                Rule::type_universal => Type::Atom(TypeAtom::Universal),
                _ => unexpected!(p),
            })
        })
        .map_infix(|l, op, r| {
            Ok(match op.as_rule() {
                Rule::type_union => Type::Union(Arc::new((l?, r?))),
                Rule::type_intersection => Type::Intersection(Arc::new((l?, r?))),
                _ => unexpected!(op),
            })
        })
        .map_postfix(|t, p| {
            t.and_then(|t| {
                Ok(match p.as_rule() {
                    Rule::type_array => Type::Array(Arc::new(t)),
                    Rule::type_dict => Type::Dict(Arc::new(t)),
                    _ => unexpected!(p),
                })
            })
        })
        .parse(p.into_inner())
}

enum Spread<T> {
    Many(Vec<T>),
    One(T),
}

/// Iterates over a vec and attempt to spread the items inside the definition of
/// which item is spreadable and how it is spread is user-defined by injecting a
/// closure `spreader` that returns `Spread` enum. The iterations are repeated until there is no more `Spread::Many` returned in one iterations.
fn spread<T>(vec: Vec<T>, spreader: impl Fn(&T) -> Spread<T>) -> Vec<T> {
    let mut spread = vec;

    loop {
        let mut something_is_spread_this_round = false;

        std::mem::take(&mut spread)
            .into_iter()
            .for_each(|item| match spreader(&item) {
                Spread::Many(v) => {
                    something_is_spread_this_round = true;
                    spread.extend(v);
                }
                Spread::One(x) => spread.push(x),
            });

        if !something_is_spread_this_round {
            break;
        }
    }

    spread
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Label, Type, TypeAtom};
    #[test]
    fn intersecting_records() {
        let a = Type::Record(
            vec![(
                Label::String("a".try_into().unwrap()),
                Type::Atom(TypeAtom::String(None)),
            )]
            .try_into()
            .unwrap(),
        );
        let b = Type::Record(
            vec![(
                Label::String("b".try_into().unwrap()),
                Type::Atom(TypeAtom::Number(None)),
            )]
            .try_into()
            .unwrap(),
        );
        let expected = Type::Record(
            vec![
                (
                    Label::String("b".try_into().unwrap()),
                    Type::Atom(TypeAtom::Number(None)),
                ),
                (
                    Label::String("a".try_into().unwrap()),
                    Type::Atom(TypeAtom::String(None)),
                ),
            ]
            .try_into()
            .unwrap(),
        );

        assert_eq!(Type::intersection([a, b]).collapse(), expected.collapse());
    }

    #[test]
    fn intersecting_identical_records() {
        let a = Type::Record(
            vec![(
                Label::String("a".try_into().unwrap()),
                Type::Atom(TypeAtom::String(None)),
            )]
            .try_into()
            .unwrap(),
        );
        assert_eq!(Type::intersection([a.clone(), a.clone()]).collapse(), a.collapse());
    }

    #[test]
    fn intersecting_conflicting_records() {
        let a = Type::Record(
            vec![(
                Label::String("a".try_into().unwrap()),
                Type::Atom(TypeAtom::String(None)),
            )]
            .try_into()
            .unwrap(),
        );
        let b = Type::Record(
            vec![(
                Label::String("a".try_into().unwrap()),
                Type::Atom(TypeAtom::Number(None)),
            )]
            .try_into()
            .unwrap(),
        );

        let res = Type::intersection([a, b]).collapse();

        assert!(
            matches!(res, Type::Never(_)),
            "res is not Type::Never, but is {:?}",
            res
        );
    }

    #[test]
    fn intersection_of_unions() {
        let union_1 = Type::union([Type::Atom(TypeAtom::Number(Some(1)))]).collapse();
        let union_2 = Type::union([
            Type::Atom(TypeAtom::Number(None)),
            Type::Atom(TypeAtom::Number(Some(1))),
        ])
        .collapse();

        assert_eq!(Type::intersection([union_1.clone(), union_2]).collapse(), union_1);
    }

    #[test]
    fn intersection_of_criss_crossing_unions() {
        let a = Type::union([
            Type::Atom(TypeAtom::Number(None)),
            Type::Atom(TypeAtom::String(Some("a".into()))),
        ]);

        let b = Type::union([
            Type::Atom(TypeAtom::Number(Some(18))),
            Type::Atom(TypeAtom::String(None)),
        ]);

        let c = Type::union([
            Type::Atom(TypeAtom::Number(Some(18))),
            Type::Atom(TypeAtom::String(None)),
            Type::Atom(TypeAtom::Null),
        ]);

        let result = Type::union([
            Type::Atom(TypeAtom::String(Some("a".to_string()))),
            Type::Atom(TypeAtom::Number(Some(18))),
        ])
        .collapse();

        //   a & (b & c)
        // = a & b // because b is subtype of c
        // = "a" | 18

        assert_eq!(
            Type::intersection([a, Type::intersection([b, c]),]).collapse(),
            result.collapse()
        );
    }

    #[test]
    fn union_collapse_sort() {
        let a = Type::union([
            Type::Atom(TypeAtom::String(None)),
            Type::Atom(TypeAtom::String(Some("asdf".to_string()))),
            Type::Atom(TypeAtom::Null),
            Type::Atom(TypeAtom::Timestamp),
        ]);

        // `b` has the same member as `a` but is structured differently
        let b = Type::union([
            Type::union([
                Type::Atom(TypeAtom::Timestamp),
                Type::Atom(TypeAtom::String(Some("asdf".to_string()))),
            ]),
            Type::union([Type::Atom(TypeAtom::Null), Type::Atom(TypeAtom::String(None))]),
        ]);

        // before collapse it isn't equal
        assert_ne!(a, b);
        // after, it is equal
        assert_eq!(a.collapse(), b.collapse());
    }

    #[test]
    fn union_collapse_singular() {
        let a = Type::union([
            Type::Atom(TypeAtom::String(None)),
            Type::Atom(TypeAtom::String(Some("asdf".to_string()))),
            Type::Atom(TypeAtom::Null),
            Type::Atom(TypeAtom::Timestamp),
        ]);

        // after, it is equal
        assert_eq!(
            Type::union(vec![a.clone(), a.clone()].into_iter()).collapse(),
            a.collapse()
        );
    }

    #[test]
    fn intersection_one_and_number() {
        assert_eq!(
            Type::intersection(
                [
                    Type::Atom(TypeAtom::Number(None)),
                    Type::Atom(TypeAtom::Number(Some(1))),
                ]
                .into_iter()
            )
            .collapse(),
            Type::Atom(TypeAtom::Number(Some(1)))
        );
    }

    #[test]
    fn union_one_and_number() {
        assert_eq!(
            Type::union(
                [
                    Type::Atom(TypeAtom::Number(None)),
                    Type::Atom(TypeAtom::Number(Some(1))),
                ]
                .into_iter()
            )
            .collapse(),
            Type::Atom(TypeAtom::Number(None))
        );
    }

    #[test]
    fn intersection_of_dict() {
        assert_eq!(
            Type::intersection(
                [
                    Type::Dict(Type::Atom(TypeAtom::Number(None)).into()),
                    Type::Dict(Type::Atom(TypeAtom::Number(Some(1))).into()),
                ]
                .into_iter()
            )
            .collapse(),
            Type::Dict(Type::Atom(TypeAtom::Number(Some(1))).into())
        );
    }

    #[test]
    fn intersection_of_array() {
        assert_eq!(
            Type::intersection(
                [
                    Type::Array(Type::Atom(TypeAtom::Number(None)).into()),
                    Type::Array(Type::Atom(TypeAtom::Number(Some(1))).into()),
                ]
                .into_iter()
            )
            .collapse(),
            Type::Array(Type::Atom(TypeAtom::Number(Some(1))).into())
        );
    }

    #[test]
    fn union_of_dict() {
        let union = Type::union(
            [
                Type::Dict(Type::Atom(TypeAtom::Number(None)).into()),
                Type::Dict(Type::Atom(TypeAtom::Number(Some(1))).into()),
            ]
            .into_iter(),
        )
        .collapse();
        assert!(
            matches!(union, Type::Union(_)),
            "unions of dictionaries shouldn't collapse despite their content hierarchy"
        );
    }

    #[test]
    fn union_of_array() {
        let union = Type::union(
            [
                Type::Array(Type::Atom(TypeAtom::Number(None)).into()),
                Type::Array(Type::Atom(TypeAtom::Number(Some(1))).into()),
            ]
            .into_iter(),
        );
        assert!(
            matches!(union, Type::Union(_)),
            "unions of array shouldn't collapse despite their content hierarchy"
        );
    }

    #[test]
    fn hierarchy_of_records_supertype() {
        // { a: { a: 1 } } is subtype of { a: { a: number, b: "someliteral"} }
        let record_a = Type::Record(
            vec![(
                Label::String("a".try_into().unwrap()),
                Type::Record(
                    vec![(
                        Label::String("a".try_into().unwrap()),
                        Type::Atom(TypeAtom::Number(None)),
                    )]
                    .try_into()
                    .unwrap(),
                ),
            )]
            .try_into()
            .unwrap(),
        )
        .collapse();
        let record_b = Type::Record(
            vec![(
                Label::String("a".try_into().unwrap()),
                Type::Record(
                    vec![
                        (
                            Label::String("a".try_into().unwrap()),
                            Type::Atom(TypeAtom::Number(Some(1))),
                        ),
                        (
                            Label::String("b".try_into().unwrap()),
                            Type::Atom(TypeAtom::String(Some("literal".to_string()))),
                        ),
                    ]
                    .try_into()
                    .unwrap(),
                ),
            )]
            .try_into()
            .unwrap(),
        )
        .collapse();
        assert_eq!(record_a.hierarchy_towards(&record_b), Hierarchy::Supertype);
        assert_eq!(record_b.hierarchy_towards(&record_a), Hierarchy::Subtype);
    }
}
