use super::{non_empty::NonEmptyString, parse_utils::P, parser::Rule};
use crate::{
    language::{
        parse_utils::{Ext, Spanned},
        parser::{r_bool, r_nonempty_string, r_string, NoVal},
    },
    NonEmptyVec,
};
use once_cell::sync::Lazy;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Type {
    Atom(TypeAtom),
    Union(Arc<(Type, Type)>),
    Intersection(Arc<(Type, Type)>),
    Array(Arc<Type>),
    Dict(Arc<Type>),
    Tuple(NonEmptyVec<Type>),
    Record(NonEmptyVec<(Label, Type)>),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum TypeAtom {
    Null,
    Bool(Option<bool>),
    Number(Option<u64>),
    Timestamp,
    String(Option<String>),
    Universal,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Label {
    String(NonEmptyString),
    Number(u64),
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
