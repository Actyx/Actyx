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
    NoValue,
    Atom(TypeAtom),
    Union(Arc<(Type, Type)>),
    Intersection(Arc<(Type, Type)>),
    Array(Arc<Type>),
    Dict(Arc<Type>),
    Tuple(NonEmptyVec<Type>),
    Record(NonEmptyVec<(Label, Type)>),
}

#[derive(Debug, PartialEq)]
pub(crate) struct CollapseError(Vec<String>);

impl CollapseError {
    fn join(errs: Vec<CollapseError>) -> CollapseError {
        CollapseError(errs.into_iter().flat_map(|x| x.0).collect())
    }
}

impl ToString for CollapseError {
    fn to_string(&self) -> String {
        self.0.clone().join(". ")
    }
}

pub enum Hierarchy {
    Disjointed,
    Equal,
    Supertype,
    Subtype,
}

impl Type {
    /// Build a union out of all types in the iterator argument
    ///
    /// # Panics
    ///
    /// Panics if the iterator does not yield any value!
    pub(crate) fn union(iter: impl DoubleEndedIterator<Item = Type>) -> Type {
        let rebuilt = iter.into_iter().rev().fold(None, |acc, next| match acc {
            None => Some(next),
            Some(prev) => Some(Type::Union(Arc::new((prev, next)))),
        });

        rebuilt.expect("union member is empty")
    }

    /// Build an intersection out of all types in the iterator argument
    ///
    /// # Panics
    ///
    /// Panics if the iterator does not yield any value!
    pub(crate) fn intersection(iter: impl DoubleEndedIterator<Item = Type>) -> Type {
        let rebuilt = iter.into_iter().rev().fold(None, |acc, next| match acc {
            None => Some(next),
            Some(prev) => Some(Type::Intersection(Arc::new((prev, next)))),
        });

        rebuilt.expect("intersection member is empty")
    }

    /// Attempt to apply collapse and sorts to intersections, unions, records
    pub(crate) fn collapse(self) -> Result<Type, CollapseError> {
        match self {
            Type::Intersection(intersection) => {
                let (a, b) = intersection.as_ref();
                let (a, b) = match (a.clone().collapse(), b.clone().collapse()) {
                    (Ok(a), Ok(b)) => (a, b),
                    (Err(a), Err(b)) => return Err(CollapseError::join(vec![a, b])),
                    (Ok(_), Err(b)) => return Err(b),
                    (Err(a), Ok(_)) => return Err(a),
                };

                if a == b {
                    return Ok(a);
                }

                match (&a, &b) {
                    // all intersections should have been collapsed
                    (Type::Intersection(_), _) | (_, Type::Intersection(_)) => Err(CollapseError(vec![
                        "intersection could not exist after a collapse".into(),
                    ])),
                    // intersection of records
                    (Type::Record(a), Type::Record(b)) => {
                        let mut errors = vec![];
                        let mut conflicts = BTreeMap::<&Label, BTreeSet<Type>>::new();
                        let mut fields = BTreeMap::<&Label, Type>::new();
                        a.iter()
                            .chain(b.iter())
                            .for_each(|(label, ty)| match ty.clone().collapse() {
                                Ok(ty) => {
                                    if !errors.is_empty() {
                                        return;
                                    }

                                    if let Some(ejected) = fields.insert(label, ty.clone()) {
                                        if ty != ejected {
                                            conflicts
                                                .entry(label)
                                                .and_modify(|set| {
                                                    set.insert(ty.clone());
                                                })
                                                .or_insert(BTreeSet::from([ejected, ty]));
                                        }
                                    }
                                }
                                Err(err) => errors.push(err),
                            });

                        if !errors.is_empty() {
                            return Err(CollapseError::join(errors));
                        }

                        if !conflicts.is_empty() {
                            return Err(CollapseError(
                                conflicts
                                    .into_iter()
                                    .map(|(label, tys)| format!("conflicting types for label {:?}: {:?}", label, tys))
                                    .collect(),
                            ));
                        }

                        Ok(Type::Record(
                            fields
                                .into_iter()
                                .map(|(label, ty)| (label.clone(), ty.clone()))
                                .collect::<Vec<(Label, Type)>>()
                                .try_into()
                                .unwrap(),
                        )
                        .collapse()?)
                    }
                    _ => match a.hierarchy_towards(&b) {
                        Hierarchy::Equal => Ok(a),
                        Hierarchy::Supertype => Ok(b),
                        Hierarchy::Subtype => Ok(a),
                        Hierarchy::Disjointed => {
                            Err(CollapseError(vec![format!("{:?} and {:?} is of different type", a, b)]))
                        }
                    },
                }
            }
            Type::Union(x) => {
                let mut collapsing = vec![];

                let flattened = Type::flatten_union_collapsing(x.as_ref())?;

                flattened.into_iter().for_each(|new| match collapsing.last_mut() {
                    None => collapsing.push(new),
                    Some(last) => {
                        match new.hierarchy_towards(last) {
                            Hierarchy::Disjointed => collapsing.push(new), // push
                            Hierarchy::Supertype => *last = new,           // replace last with supertype
                            Hierarchy::Subtype | Hierarchy::Equal => {}    // do nothing
                        }
                    }
                });

                Ok(Type::union(collapsing.into_iter()))
            }
            Type::Record(fields) => {
                let mut errors = vec![];
                let fields = fields
                    .iter()
                    .cloned()
                    .filter_map(|(label, ty)| match ty.collapse() {
                        Ok(ty) => Some((label, ty)),
                        Err(err) => {
                            errors.push(err);
                            None
                        }
                    })
                    .collect::<BTreeMap<_, _>>();

                if !errors.is_empty() {
                    Err(CollapseError::join(errors))
                } else {
                    Ok(Type::Record(fields.into_iter().collect::<Vec<_>>().try_into().unwrap()))
                }
            }
            _ => Ok(self),
        }
    }

    fn hierarchy_towards(&self, other: &Type) -> Hierarchy {
        if self == other {
            return Hierarchy::Equal;
        }

        match (self, other) {
            (Type::Atom(a), Type::Atom(b)) => match (a, b) {
                // supertypes
                (TypeAtom::Bool(None), TypeAtom::Bool(Some(_))) => Hierarchy::Supertype,
                (TypeAtom::Number(None), TypeAtom::Number(Some(_))) => Hierarchy::Supertype,
                (TypeAtom::String(None), TypeAtom::String(Some(_))) => Hierarchy::Supertype,
                // subtypes
                (TypeAtom::Bool(Some(_)), TypeAtom::Bool(None)) => Hierarchy::Subtype,
                (TypeAtom::Number(Some(_)), TypeAtom::Number(None)) => Hierarchy::Subtype,
                (TypeAtom::String(Some(_)), TypeAtom::String(None)) => Hierarchy::Subtype,
                (TypeAtom::Universal, TypeAtom::Universal) => Hierarchy::Equal,
                (TypeAtom::Universal, _) => Hierarchy::Supertype,
                _ => Hierarchy::Disjointed,
            },
            (Type::Record(a), Type::Record(b)) => {
                let a = a.iter().collect::<BTreeSet<_>>();
                let b = b.iter().collect::<BTreeSet<_>>();
                let intersection = BTreeSet::intersection(&a, &b).cloned().collect::<BTreeSet<_>>();

                if intersection == b {
                    Hierarchy::Supertype
                } else if intersection == a {
                    Hierarchy::Subtype
                } else {
                    Hierarchy::Disjointed
                }
            }
            (Type::Union(a), Type::Union(b)) => {
                let a = Type::flatten_union(a.as_ref());
                let b = Type::flatten_union(b.as_ref());
                let intersection = BTreeSet::intersection(&a, &b).cloned().collect::<BTreeSet<_>>();

                if intersection == b {
                    Hierarchy::Supertype
                } else if intersection == a {
                    Hierarchy::Subtype
                } else {
                    Hierarchy::Disjointed
                }
            }
            (Type::Union(union), non_union) if Type::flatten_union(union.as_ref()).contains(non_union) => {
                Hierarchy::Supertype
            }
            (non_union, Type::Union(union)) if Type::flatten_union(union.as_ref()).contains(non_union) => {
                Hierarchy::Subtype
            }
            (Type::Array(a), Type::Array(b)) => a.hierarchy_towards(b),
            (Type::Dict(a), Type::Dict(b)) => a.hierarchy_towards(b),
            _ => Hierarchy::Disjointed,
        }
    }

    /// Flatten union tree into a set of type references without collapsing
    fn flatten_union((a, b): &(Type, Type)) -> BTreeSet<&Type> {
        let mut under_types_vec = vec![a, b];

        loop {
            let mut non_unions = vec![];
            let mut unions = vec![];

            under_types_vec.split_off(0).into_iter().for_each(|ty| match ty {
                Type::Union(pair) => unions.push(pair),
                _ => non_unions.push(ty),
            });
            std::mem::swap(&mut under_types_vec, &mut non_unions);

            if unions.is_empty() {
                break;
            }

            under_types_vec.extend(unions.into_iter().flat_map(|x| {
                let (a, b) = x.as_ref();
                [a, b]
            }));
        }

        under_types_vec.into_iter().collect()
    }

    /// Flatten union tree into a set of types with collapsing
    fn flatten_union_collapsing((a, b): &(Type, Type)) -> Result<BTreeSet<Type>, CollapseError> {
        let mut under_types_vec = vec![a.clone(), b.clone()];
        let mut collapse_errors = vec![];

        loop {
            let mut non_unions = vec![];
            let mut unions = vec![];

            std::mem::take(&mut under_types_vec)
                .into_iter()
                .for_each(|item| match item.collapse() {
                    Ok(Type::Union(pair)) => unions.push(pair),
                    Ok(ty) => non_unions.push(ty),
                    Err(err) => collapse_errors.push(err),
                });
            std::mem::swap(&mut under_types_vec, &mut non_unions);

            if !collapse_errors.is_empty() {
                return Err(CollapseError::join(collapse_errors));
            }

            if unions.is_empty() {
                break;
            }

            under_types_vec.extend(unions.into_iter().flat_map(|x| {
                let (a, b) = x.as_ref();
                [a.clone(), b.clone()]
            }));
        }

        Ok(under_types_vec.into_iter().collect())
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

#[cfg(test)]
mod tests {
    use crate::{Label, Type, TypeAtom};
    use std::sync::Arc;
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

        assert_eq!(
            Type::Intersection(Arc::new((a, b))).collapse(),
            Ok(expected.collapse().expect("should not err"))
        );
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
        assert_eq!(Type::Intersection(Arc::new((a.clone(), a.clone()))).collapse(), Ok(a));
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
        assert!(Type::Intersection(Arc::new((a, b))).collapse().is_err());
    }

    #[test]
    fn union_collapse_sort() {
        let a = Type::union(
            vec![
                Type::Atom(TypeAtom::String(None)),
                Type::Atom(TypeAtom::String(Some("asdf".to_string()))),
                Type::Atom(TypeAtom::Null),
                Type::Atom(TypeAtom::Timestamp),
            ]
            .into_iter(),
        );

        // `b` has the same member as `a` but is structured differently
        let b = Type::union(
            vec![
                Type::union(
                    vec![
                        Type::Atom(TypeAtom::Timestamp),
                        Type::Atom(TypeAtom::String(Some("asdf".to_string()))),
                    ]
                    .into_iter(),
                ),
                Type::union(vec![Type::Atom(TypeAtom::Null), Type::Atom(TypeAtom::String(None))].into_iter()),
            ]
            .into_iter(),
        );

        // before collapse it isn't equal
        assert_ne!(a, b);
        // after, it is equal
        assert_eq!(a.collapse().unwrap(), b.collapse().unwrap());
    }

    #[test]
    fn union_collapse_singular() {
        let a = Type::union(
            vec![
                Type::Atom(TypeAtom::String(None)),
                Type::Atom(TypeAtom::String(Some("asdf".to_string()))),
                Type::Atom(TypeAtom::Null),
                Type::Atom(TypeAtom::Timestamp),
            ]
            .into_iter(),
        );

        // after, it is equal
        assert_eq!(
            Type::union(vec![a.clone(), a.clone()].into_iter()).collapse().unwrap(),
            a.collapse().unwrap()
        );
    }

    #[test]
    fn intersection_one_and_number() {
        assert_eq!(
            Type::intersection(
                vec![
                    Type::Atom(TypeAtom::Number(None)),
                    Type::Atom(TypeAtom::Number(Some(1))),
                ]
                .into_iter()
            )
            .collapse()
            .unwrap(),
            Type::Atom(TypeAtom::Number(Some(1)))
        );
    }

    #[test]
    fn union_one_and_number() {
        assert_eq!(
            Type::union(
                vec![
                    Type::Atom(TypeAtom::Number(None)),
                    Type::Atom(TypeAtom::Number(Some(1))),
                ]
                .into_iter()
            )
            .collapse()
            .unwrap(),
            Type::Atom(TypeAtom::Number(None))
        );
    }
}
