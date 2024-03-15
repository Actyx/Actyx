use super::workflow::{Participant, Workflow, WorkflowStep};
use crate::{Ident, Ind, Index, Label, NonEmptyVec, Num, Query, SimpleExpr, Type, TypeAtom};
use std::{collections::BTreeSet, ops::Deref, sync::Arc};
pub(crate) struct QueryTypeCheckError<'a> {
    pub(crate) span: pest::Span<'a>,
    pub(crate) error_type: QueryTypeCheckErrorType,
}

type ExtraMessage = String;
pub(crate) enum QueryTypeCheckErrorType {
    UndeclaredEvent,
    UndeclaredParticipant,
    UndeclaredWorkflowCall,
    InvalidIndex(ExtraMessage),
}
struct WorkflowAnalysisResource<'a> {
    workflow: &'a Workflow<'a>,
    participants_set: BTreeSet<&'a Ident>,
}

impl<'a> WorkflowAnalysisResource<'a> {
    fn from(workflow: &'a Workflow) -> Self {
        Self {
            workflow,
            participants_set: workflow
                .args
                .iter()
                .map(|x| {
                    let participant: &Participant = x;
                    match participant {
                        Participant::Role(ident) => ident,
                        Participant::Unique(ident) => ident,
                    }
                })
                .collect(),
        }
    }
}

pub(crate) struct QueryTypeCheck<'a> {
    pub(crate) query: &'a Query<'a>,
}

impl<'a> QueryTypeCheck<'a> {
    pub fn from(query: &'a Query) -> Self {
        Self { query }
    }

    pub fn check(&'a self) -> Vec<QueryTypeCheckError> {
        self.query
            .workflows
            .iter()
            .flat_map(|(_ident, workflow)| self.check_workflow(workflow))
            .collect()
    }

    fn check_workflow(&'a self, workflow: &'a Workflow) -> Vec<QueryTypeCheckError> {
        self.check_steps(&WorkflowAnalysisResource::from(workflow), &workflow.steps)
    }

    fn check_steps(
        &'a self,
        parent_workflow: &WorkflowAnalysisResource,
        steps: &'a NonEmptyVec<WorkflowStep<'a>>,
    ) -> Vec<QueryTypeCheckError> {
        steps
            .iter()
            .flat_map(|step| self.check_step(parent_workflow, step))
            .collect()
    }

    fn check_step(
        &'a self,
        parent_workflow: &WorkflowAnalysisResource,
        step: &'a WorkflowStep<'a>,
    ) -> Vec<QueryTypeCheckError> {
        use super::workflow::WorkflowStep as W;

        match step {
            W::Event { label, binders, .. } => {
                let mut errors = Vec::new();
                let matching_event = self.query.events.get(label);

                if matching_event.is_none() {
                    errors.push(QueryTypeCheckError {
                        error_type: QueryTypeCheckErrorType::UndeclaredEvent,
                        span: label.span(),
                    });
                }

                if let Some((ev_type, _tag)) = matching_event {
                    binders.iter().for_each(|binding| {
                        let value = binding.value();

                        let res: Result<(), ExtraMessage> = match value {
                            SimpleExpr::Indexing(ind) => drill_down_type_by_index(ev_type, ind).and(Ok(())),
                            // SimpleExpr::Variable(_) => todo!(),
                            // SimpleExpr::Number(_) => todo!(),
                            // SimpleExpr::String(_) => todo!(),
                            // SimpleExpr::Interpolation(_) => todo!(),
                            // SimpleExpr::Object(_) => todo!(),
                            // SimpleExpr::Array(_) => todo!(),
                            // SimpleExpr::Null => todo!(),
                            // SimpleExpr::Bool(_) => todo!(),
                            // SimpleExpr::Cases(_) => todo!(),
                            // SimpleExpr::BinOp(_) => todo!(),
                            // SimpleExpr::Not(_) => todo!(),
                            // SimpleExpr::AggrOp(_) => todo!(),
                            // SimpleExpr::FuncCall(_) => todo!(),
                            // SimpleExpr::SubQuery(_) => todo!(),
                            // SimpleExpr::KeyVar(_) => todo!(),
                            // SimpleExpr::KeyLiteral(_) => todo!(),
                            // SimpleExpr::TimeVar(_) => todo!(),
                            // SimpleExpr::TimeLiteral(_) => todo!(),
                            // SimpleExpr::Tags(_) => todo!(),
                            // SimpleExpr::App(_) => todo!(),
                            _ => Ok(()),
                        };

                        if let Err(error_message) = res {
                            errors.push(QueryTypeCheckError {
                                span: binding.span(),
                                error_type: QueryTypeCheckErrorType::InvalidIndex(error_message),
                            })
                        }
                    });
                }

                // Check binders for undeclared participants
                errors.extend(binders.iter().filter_map(|binding_span| {
                    if !parent_workflow.participants_set.contains(binding_span.role()) {
                        Some(QueryTypeCheckError {
                            error_type: QueryTypeCheckErrorType::UndeclaredParticipant,
                            span: binding_span.span(),
                        })
                    } else {
                        None
                    }
                }));

                // this part will also handle additional stuffs like type matching in `binders`
                errors
            }
            W::Retry { steps } => self.check_steps(parent_workflow, steps),
            W::Timeout { steps, .. } => self.check_steps(parent_workflow, steps),
            W::Parallel { cases, .. } => cases
                .iter()
                .flat_map(|case| self.check_steps(parent_workflow, case))
                .collect(),
            W::Call {
                workflow: workflow_ident,
                cases,
                ..
            } => {
                let mut errors = Vec::new();
                if self.query.workflows.get(workflow_ident).is_none() {
                    errors.push(QueryTypeCheckError {
                        error_type: QueryTypeCheckErrorType::UndeclaredWorkflowCall,
                        span: workflow_ident.span(),
                    });
                }

                errors.extend(
                    cases
                        .iter()
                        .flat_map(|(_, steps)| self.check_steps(parent_workflow, steps)),
                );

                errors
            }
            W::Compensate { body, with } => std::iter::empty()
                .chain(self.check_steps(parent_workflow, body))
                .chain(self.check_steps(parent_workflow, with))
                .collect(),
            W::Choice { cases } => cases
                .iter()
                .flat_map(|steps| self.check_steps(parent_workflow, steps))
                .collect(),
        }
    }
}

/// Check whether Ind can be used to index a Type
/// e.g. can _.a.b be used to access Type::Record([("a", Type::Record([("b", Type::String)]))])
/// Drills down type and Ind.tail recursively at the same time to find a match
fn drill_down_type_by_index(cur_type: &Type, Ind { head: _, tail, .. }: &Ind) -> Result<Type, ExtraMessage> {
    fn recurse(cur_type: &Type, index: &[Index]) -> Result<Type, ExtraMessage> {
        if index.len() < 1 {
            return Ok(cur_type.to_owned());
        }
        let (first, rest) = index.split_at(1);
        let first = &first[0];

        match cur_type {
            Type::Atom(type_atom) => match type_atom {
                TypeAtom::Universal => return Ok(Type::Atom(TypeAtom::Universal)),
                TypeAtom::Tuple(tuple) => match first {
                    Index::Expr(expr) => recurse_with_calculated_expression(cur_type, expr, rest),
                    Index::Number(num_index) => tuple.get(*num_index as usize).map_or_else(
                        || Err(format!("{:?} cannot be accessed by {}", tuple, num_index)),
                        |ty| recurse(ty, rest),
                    ),
                    _ => Err(format!("{:?} cannot be accessed by {:?}", tuple, first)),
                },
                TypeAtom::Record(fields) => {
                    if let Index::Expr(expr) = first {
                        return recurse_with_calculated_expression(cur_type, expr, rest);
                    }

                    let matching_field = fields.iter().find(|(label, _)| match (label, first) {
                        (Label::String(a), Index::String(b)) => Deref::deref(a) == b,
                        (Label::Number(a), Index::Number(b)) => a == b,
                        _ => false,
                    });

                    if let Some((_, ty)) = matching_field {
                        recurse(ty, rest)
                    } else {
                        Err(format!("{:?} cannot be accessed by {:?}", type_atom, first))
                    }
                }
                _ => Err(format!("{:?} cannot be accessed by {:?}", type_atom, first)),
            },
            Type::Dict(ty) => match first {
                Index::Expr(expr) => recurse_with_calculated_expression(cur_type, expr, rest),
                Index::String(_) => recurse(ty, rest),
                _ => Err(format!("{:?} cannot be accessed by a non-string", cur_type)),
            },
            Type::Array(ty) => match first {
                Index::Expr(expr) => recurse_with_calculated_expression(cur_type, expr, rest),
                Index::Number(_) => recurse(ty, rest),
                _ => Err(format!("{:?} cannot be accessed by a non-number", cur_type)),
            },
            Type::Union(ty) => {
                let a_result = recurse(&ty.0, index);
                let b_result = recurse(&ty.1, index);

                match (a_result, b_result) {
                    (Ok(a), Ok(b)) => {
                        let mut new_union = Type::Union(Arc::new((a, b)));
                        new_union.collapse_union();
                        Ok(new_union)
                    }
                    (Ok(_), Err(b)) => Err(b),
                    (Err(a), Ok(_)) => Err(a),
                    (Err(a), Err(b)) => Err([a, b]
                        .into_iter()
                        .map(|err_msg| format!("{}", err_msg))
                        .collect::<Vec<_>>()
                        .join("\n")),
                }
            }
            Type::Intersection(_) => {
                // NOTE: impossible to drill down on intersection without intersection collapsing mechanism
                Err(format!("{:?} cannot be accessed", cur_type))
            }
        }
    }

    /// attempt drill down on a type using a calculated expression
    fn recurse_with_calculated_expression(
        cur_type: &Type,
        first_expr: &SimpleExpr,
        rest: &[Index],
    ) -> Result<Type, ExtraMessage> {
        let new_index = match first_expr {
            SimpleExpr::Number(Num::Natural(x)) => Index::Number(x.clone()),
            SimpleExpr::String(x) => Index::String(x.clone()),
            // Note: Not sure if rounded Num::Decimal should be regarded as natural or not
            SimpleExpr::Number(Num::Decimal(_))
            | SimpleExpr::Bool(_)
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
            | SimpleExpr::BinOp(_) => return Err(format!("{:?} cannot be accessed by {}", cur_type, first_expr)),
            SimpleExpr::Variable(_)
            | SimpleExpr::Indexing(_)
            | SimpleExpr::Interpolation(_)
            | SimpleExpr::AggrOp(_)
            | SimpleExpr::FuncCall(_)
            | SimpleExpr::SubQuery(_) => return Ok(Type::Atom(TypeAtom::Universal)),
            SimpleExpr::Cases(c) => {
                // Case acts similarly to a union because it may results in several types

                let type_results = c
                    .iter()
                    .map(|(_, then)| recurse_with_calculated_expression(cur_type, then, rest))
                    .collect::<Vec<_>>();

                let errors = type_results
                    .iter()
                    .filter_map(|x| match x {
                        Ok(_) => None,
                        Err(err_msg) => Some(err_msg),
                    })
                    .collect::<Vec<_>>();

                if !errors.is_empty() {
                    let mut error_messages = String::new();
                    errors
                        .into_iter()
                        .map(|err_msg| format!("{}", err_msg))
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
                        Some(a) => Some(Type::Union(Arc::new((a, item)))),
                    });

                let mut union =
                    union.expect("the type_results to be non-empty because it is derived from a non-empty cases");
                union.collapse_union();

                return Ok(union);
            }
        };

        recurse(cur_type, [&[new_index], &rest[..]].concat().as_slice())
    }

    recurse(cur_type, tail)
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
            Ok(Type::Atom(TypeAtom::Number(None)))
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
            Ok(Type::Atom(TypeAtom::Number(None)))
        );

        assert!(drill_down_type_by_index(
            &Type::Dict(Arc::new(Type::Atom(TypeAtom::Number(None)))),
            &Ind {
                head: SimpleExpr::Variable(Var("_".to_string())).into(),
                tail: vec![Index::Number(0_u64)].try_into().unwrap(),
            },
        )
        .is_err());
    }

    #[test]
    fn record() {
        assert_eq!(
            drill_down_type_by_index(
                &Type::Atom(TypeAtom::Record(
                    vec![(
                        Label::String("a".try_into().unwrap()),
                        Type::Atom(TypeAtom::Record(
                            vec![(
                                Label::String("b".try_into().unwrap()),
                                Type::Atom(TypeAtom::Record(
                                    vec![(Label::Number(1), Type::Atom(TypeAtom::Null))].try_into().unwrap()
                                ))
                            )]
                            .try_into()
                            .unwrap()
                        ))
                    )]
                    .try_into()
                    .unwrap()
                )),
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
        let tuple_type = Type::Atom(TypeAtom::Tuple(
            vec![
                Type::Atom(TypeAtom::Null),
                Type::Atom(TypeAtom::String(None)),
                Type::Atom(TypeAtom::Number(None)),
            ]
            .try_into()
            .unwrap(),
        ));

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

        let record_type = Type::Atom(TypeAtom::Record(
            vec![
                (
                    Label::String("a".try_into().unwrap()),
                    Type::Atom(TypeAtom::Bool(Some(false))),
                ),
                (Label::Number(0), Type::Atom(TypeAtom::Bool(Some(true)))),
            ]
            .try_into()
            .unwrap(),
        ));

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

        assert!(
            drill_down_type_by_index(
                &record_type,
                &Ind {
                    head: SimpleExpr::Variable(Var("_".to_string())).into(),
                    tail: vec![expr_number_decimal_0].try_into().unwrap(),
                },
            ).is_err()
        );

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
