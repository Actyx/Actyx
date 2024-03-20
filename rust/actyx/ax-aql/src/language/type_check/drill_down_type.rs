use std::{ops::Deref, sync::Arc};

use crate::{Ind, Index, Label, Num, SimpleExpr, Type, TypeAtom};

/// Check whether Ind can be used to index a Type
/// e.g. can _.a.b be used to access Type::Record([("a", Type::Record([("b", Type::String)]))])
/// Drills down type and Ind.tail recursively at the same time to find a match
pub(crate) fn drill_down_type_by_index(current_type: &Type, ind: &Ind) -> Result<Type, String> {
    // Define drill_down definitions
    // Developer Notes:
    // catch-all case _ => is avoided to prevent bugs due to missing cases when
    // a new Type, TypeAtom, or SimpleExpr is introduced

    /// Drills down type by an index slice
    /// Developer Notes:
    /// this functions operate in 2 modes: a loop and a recursion
    /// - the loop pop_front the index until the index is empty, which then will return Ok(current_type)
    /// - in case loop isn't possible, return its recursion.
    /// - in both cases, this function relies on explicit returns, pay attention to when and where.
    fn drill_down(current_type: &Type, index: &[Index]) -> Result<Type, String> {
        let mut current_type = current_type.clone();
        let mut index = index.clone();

        loop {
            if index.is_empty() {
                return Ok(current_type);
            }
            let first = &index[0];
            let rest = &index[1..];

            current_type = match &current_type {
                Type::NoValue => return Err(format!("{:?} cannot be accessed.", current_type)),
                Type::Tuple(tuple) => match first {
                    Index::Expr(expr) => return drill_down_with_calculated_expressions(&current_type, expr, rest),
                    Index::Number(num_index) => match tuple.get(*num_index as usize) {
                        None => return Err(format!("{:?} cannot be accessed by {}", tuple, num_index)),
                        Some(ty) => ty.clone(),
                    },
                    _ => return Err(format!("{:?} cannot be accessed by {:?}", tuple, first)),
                },
                Type::Record(fields) => {
                    if let Index::Expr(expr) = first {
                        return drill_down_with_calculated_expressions(&current_type, expr, rest);
                    }

                    let matching_field = fields.iter().find(|(label, _)| match (label, first) {
                        (Label::String(a), Index::String(b)) => Deref::deref(a) == b,
                        (Label::Number(a), Index::Number(b)) => a == b,
                        _ => false,
                    });

                    match matching_field {
                        None => return Err(format!("{:?} cannot be accessed by {:?}", current_type, first)),
                        Some((_, ty)) => ty.clone(),
                    }
                }
                Type::Atom(type_atom) => match type_atom {
                    TypeAtom::Universal => return Ok(Type::Union(Arc::from((current_type, Type::NoValue)))),
                    TypeAtom::Null
                    | TypeAtom::Bool(_)
                    | TypeAtom::Number(_)
                    | TypeAtom::Timestamp
                    | TypeAtom::String(_) => return Err(format!("{:?} cannot be accessed by {:?}", type_atom, first)),
                },
                Type::Dict(ty) => match first {
                    Index::Number(_) => Type::Union(Arc::from((ty.as_ref().clone(), Type::NoValue))),
                    Index::String(_) => Type::Union(Arc::from((ty.as_ref().clone(), Type::NoValue))),
                    Index::Expr(expr) => {
                        return drill_down_with_calculated_expressions(&current_type, expr, rest)
                            .map(|ty| Type::Union(Arc::from((ty, Type::NoValue))))
                            .and_then(|ty| ty.collapse().map_err(|e| e.to_string()))
                    }
                },
                Type::Array(ty) => match first {
                    Index::Number(_) => Type::Union(Arc::from((ty.as_ref().clone(), Type::NoValue))),
                    Index::Expr(expr) => {
                        return drill_down_with_calculated_expressions(&current_type, expr, rest)
                            .map(|ty| Type::Union(Arc::from((ty, Type::NoValue))))
                            .and_then(|ty| ty.collapse().map_err(|e| e.to_string()))
                    }
                    Index::String(_) => return Err(format!("{:?} cannot be accessed by a non-number", current_type)),
                },
                Type::Union(ty) => {
                    let a_result = drill_down(&ty.0, index);
                    let b_result = drill_down(&ty.1, index);

                    return match (a_result, b_result) {
                        (Ok(a), Ok(b)) => {
                            let new_union = Type::Union(Arc::new((a, b))).collapse().map_err(|e| e.to_string())?;
                            Ok(new_union)
                        }
                        (Ok(_), Err(b)) => Err(b),
                        (Err(a), Ok(_)) => Err(a),
                        (Err(a), Err(b)) => {
                            let joined_error_messages = [a, b]
                                .into_iter()
                                .map(|err_msg| format!("- {}", err_msg))
                                .collect::<Vec<_>>()
                                .join("\n");

                            Err(joined_error_messages)
                        }
                    };
                }
                Type::Intersection(_) => {
                    let collapsed = current_type.clone().collapse().map_err(|e| e.to_string())?;
                    return drill_down(&collapsed, index);
                }
            };

            // index pop_front-ing
            index = rest;
        }
    }

    fn drill_down_with_calculated_expressions(
        current_type: &Type,
        first_expr: &SimpleExpr,
        rest: &[Index],
    ) -> Result<Type, String> {
        let new_index = match first_expr {
            SimpleExpr::Number(Num::Natural(x)) => Index::Number(*x),
            SimpleExpr::String(x) => Index::String(x.clone()),
            SimpleExpr::Number(Num::Decimal(_)) => {
                return Err(format!(
                    "{:?} cannot be accessed by {}. Decimal cannot be used for indexing.",
                    current_type, first_expr
                ))
            }
            SimpleExpr::Bool(_)
            | SimpleExpr::Object(_)
            | SimpleExpr::Array(_)
            | SimpleExpr::Null
            | SimpleExpr::Not(_)
            | SimpleExpr::KeyVar(_)
            | SimpleExpr::KeyLiteral(_)
            | SimpleExpr::TimeVar(_)
            | SimpleExpr::TimeLiteral(_)
            | SimpleExpr::Tags(_)
            | SimpleExpr::App(_)
            | SimpleExpr::BinOp(_) => return Err(format!("{:?} cannot be accessed by {}", current_type, first_expr)),
            SimpleExpr::Indexing(_) => {
                // TODO: support
                return Err(format!(
                    "{:?} cannot be accessed by {:?}. Indexing by an indexing expression isn't supported yet",
                    current_type, first_expr
                ));
            }
            SimpleExpr::Variable(_)
            | SimpleExpr::Interpolation(_)
            | SimpleExpr::AggrOp(_)
            | SimpleExpr::FuncCall(_)
            | SimpleExpr::SubQuery(_) => return drill_down(&Type::Atom(TypeAtom::Universal), rest),
            SimpleExpr::Cases(c) => {
                // Case acts similarly to a union because it may results in several types

                let type_results = c
                    .iter()
                    .map(|(_, then)| drill_down_with_calculated_expressions(current_type, then, rest))
                    .collect::<Vec<_>>();

                let errors = type_results
                    .iter()
                    .filter_map(|x| match x {
                        Ok(_) => None,
                        Err(err_msg) => Some(err_msg),
                    })
                    .collect::<Vec<_>>();

                if !errors.is_empty() {
                    let error_messages = errors
                        .into_iter()
                        .map(|err_msg| format!("- {}", err_msg))
                        .collect::<Vec<_>>()
                        .join("\n");
                    return Err(error_messages.trim().into());
                }

                // NOTE: Using Option to emulate bottom type; consider introducing bottom type
                let union = type_results
                    .into_iter()
                    .filter_map(|x| match x {
                        Ok(x) => Some(x),
                        Err(_) => None,
                    })
                    .fold(None, |acc, item| match acc {
                        None => Some(item),
                        Some(prev) => Some(Type::Union(Arc::new((prev, item)))),
                    });

                let union = union
                    .expect("the type_results to be non-empty because it is derived from a non-empty cases")
                    .collapse()
                    .map_err(|e| e.to_string())?;

                return Ok(union);
            }
        };

        drill_down(current_type, [&[new_index], rest].concat().as_slice())
    }

    // start drilling_down
    let Ind { head: _, tail, .. } = ind;
    let result = drill_down(current_type, tail);

    match result {
        Ok(x) => Ok(x.collapse().map_err(|e| e.to_string())?),
        Err(x) => {
            if tail.len() > 1 {
                // create a root-level error message
                let root_lv_err = format!("{:?} cannot be accessed by {:?}", current_type, ind);
                // append with indented deeper_lv_err
                let deeper_lv_err = x
                    .split('\n')
                    .map(|line| format!("  {:?}", line))
                    .collect::<Vec<_>>()
                    .join("\n");

                Err([root_lv_err, deeper_lv_err].join("\n"))
            } else {
                Err(x)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{TypeAtom, Var};

    #[test]
    fn array_by_number() {
        assert_eq!(
            drill_down_type_by_index(
                &Type::Array(Arc::new(Type::Atom(TypeAtom::Number(None)))),
                &Ind {
                    head: SimpleExpr::Variable(Var("_".to_string())).into(),
                    tail: vec![Index::Number(0_u64)].try_into().unwrap(),
                },
            ),
            Ok(Type::Union(Arc::new((
                Type::Atom(TypeAtom::Number(None)),
                Type::NoValue
            ))))
        );

        assert!(drill_down_type_by_index(
            &Type::Array(Arc::new(Type::Atom(TypeAtom::Number(None)))),
            &Ind {
                head: SimpleExpr::Variable(Var("_".to_string())).into(),
                tail: vec![Index::String(String::from("some_string"))].try_into().unwrap(),
            },
        )
        .is_err());
    }

    #[test]
    fn dict() {
        assert_eq!(
            drill_down_type_by_index(
                &Type::Dict(Arc::new(Type::Atom(TypeAtom::Number(None)))),
                &Ind {
                    head: SimpleExpr::Variable(Var("_".to_string())).into(),
                    tail: vec![Index::String(String::from("some_string"))].try_into().unwrap(),
                },
            ),
            Ok(Type::Union(Arc::new((
                Type::Atom(TypeAtom::Number(None)),
                Type::NoValue
            ))))
        );
        assert_eq!(
            drill_down_type_by_index(
                &Type::Dict(Arc::new(Type::Atom(TypeAtom::Number(None)))),
                &Ind {
                    head: SimpleExpr::Variable(Var("_".to_string())).into(),
                    tail: vec![Index::Number(1)].try_into().unwrap(),
                },
            ),
            Ok(Type::Union(Arc::new((
                Type::Atom(TypeAtom::Number(None)),
                Type::NoValue
            ))))
        );
    }

    #[test]
    fn record() {
        assert_eq!(
            drill_down_type_by_index(
                &Type::Record(
                    vec![(
                        Label::String("a".try_into().unwrap()),
                        Type::Record(
                            vec![(
                                Label::String("b".try_into().unwrap()),
                                Type::Record(vec![(Label::Number(1), Type::Atom(TypeAtom::Null))].try_into().unwrap())
                            )]
                            .try_into()
                            .unwrap()
                        )
                    )]
                    .try_into()
                    .unwrap()
                ),
                &Ind {
                    head: SimpleExpr::Variable(Var("_".to_string())).into(),
                    tail: vec![
                        Index::String("a".to_string()),
                        Index::String("b".to_string()),
                        Index::Number(1)
                    ]
                    .try_into()
                    .unwrap(),
                },
            ),
            Ok(Type::Atom(TypeAtom::Null))
        );
    }

    #[test]
    fn tuple() {
        let tuple_type = Type::Tuple(
            vec![
                Type::Atom(TypeAtom::Null),
                Type::Atom(TypeAtom::String(None)),
                Type::Atom(TypeAtom::Number(None)),
            ]
            .try_into()
            .unwrap(),
        );

        assert_eq!(
            drill_down_type_by_index(
                &tuple_type,
                &Ind {
                    head: SimpleExpr::Variable(Var("_".to_string())).into(),
                    tail: vec![Index::Number(0)].try_into().unwrap(),
                },
            ),
            Ok(Type::Atom(TypeAtom::Null))
        );

        assert_eq!(
            drill_down_type_by_index(
                &tuple_type,
                &Ind {
                    head: SimpleExpr::Variable(Var("_".to_string())).into(),
                    tail: vec![Index::Number(1)].try_into().unwrap(),
                },
            ),
            Ok(Type::Atom(TypeAtom::String(None)))
        );

        assert_eq!(
            drill_down_type_by_index(
                &tuple_type,
                &Ind {
                    head: SimpleExpr::Variable(Var("_".to_string())).into(),
                    tail: vec![Index::Number(2)].try_into().unwrap(),
                },
            ),
            Ok(Type::Atom(TypeAtom::Number(None)))
        );

        assert!(drill_down_type_by_index(
            &tuple_type,
            &Ind {
                head: SimpleExpr::Variable(Var("_".to_string())).into(),
                tail: vec![Index::Number(3)].try_into().unwrap(),
            },
        )
        .is_err());
    }

    #[test]
    fn expr_as_index() {
        let expr_number_natural_0 = Index::Expr(SimpleExpr::Number(Num::Natural(0)));
        let expr_number_decimal_0 = Index::Expr(SimpleExpr::Number(Num::Decimal(0.0)));
        let expr_string_a = Index::Expr(SimpleExpr::String("a".into()));
        let expr_anyother = Index::Expr(SimpleExpr::Variable(Var("somevar".into())));

        let record_type = Type::Record(
            vec![
                (
                    Label::String("a".try_into().unwrap()),
                    Type::Atom(TypeAtom::Bool(Some(false))),
                ),
                (Label::Number(0), Type::Atom(TypeAtom::Bool(Some(true)))),
            ]
            .try_into()
            .unwrap(),
        );

        assert_eq!(
            drill_down_type_by_index(
                &record_type,
                &Ind {
                    head: SimpleExpr::Variable(Var("_".to_string())).into(),
                    tail: vec![expr_number_natural_0].try_into().unwrap(),
                },
            ),
            Ok(Type::Atom(TypeAtom::Bool(Some(true))))
        );

        assert_eq!(
            drill_down_type_by_index(
                &record_type,
                &Ind {
                    head: SimpleExpr::Variable(Var("_".to_string())).into(),
                    tail: vec![expr_string_a].try_into().unwrap(),
                },
            ),
            Ok(Type::Atom(TypeAtom::Bool(Some(false))))
        );

        assert!(drill_down_type_by_index(
            &record_type,
            &Ind {
                head: SimpleExpr::Variable(Var("_".to_string())).into(),
                tail: vec![expr_number_decimal_0].try_into().unwrap(),
            },
        )
        .is_err());

        assert_eq!(
            drill_down_type_by_index(
                &record_type,
                &Ind {
                    head: SimpleExpr::Variable(Var("_".to_string())).into(),
                    tail: vec![expr_anyother].try_into().unwrap(),
                },
            ),
            Ok(Type::Atom(TypeAtom::Universal))
        );
    }
}
