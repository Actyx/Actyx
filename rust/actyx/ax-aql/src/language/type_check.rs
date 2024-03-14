use super::workflow::{Participant, Workflow, WorkflowStep};
use crate::{Ident, Ind, Index, NonEmptyVec, Query, SimpleExpr, Type};
use std::{collections::BTreeSet, ops::Deref};

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

                        let res = match value {
                            SimpleExpr::Indexing(ind) => index_tail_access_matches(ev_type, ind),
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
fn index_tail_access_matches(cur_type: &Type, Ind { tail, .. }: &Ind) -> Result<(), ExtraMessage> {
    fn recurse(cur_type: &Type, index: &[Index]) -> Result<(), ExtraMessage> {
        let (first, rest) = index.split_at(1);
        if first.is_empty() {
            return Ok(());
        }

        let first = &first[0];

        match cur_type {
            Type::Atom(x) => match x {
                crate::TypeAtom::Universal => Ok(()),
                crate::TypeAtom::Tuple(x) => match first {
                    Index::Number(num_index) => {
                        if (*num_index as usize) < x.len() {
                            Ok(())
                        } else {
                            Err(format!("{:?} cannot be accessed by {}", x, num_index))
                        }
                    }
                    _ => Err(format!("{:?} cannot be accessed by {:?}", x, first)),
                },
                crate::TypeAtom::Record(fields) => {
                    let matching_field = fields.iter().find(|(label, _)| match (label, first) {
                        (crate::Label::String(a), Index::String(b)) => Deref::deref(a) == b,
                        (crate::Label::Number(a), Index::Number(b)) => a == b,
                        _ => false,
                    });

                    if let Some((_, ty)) = matching_field {
                        recurse(ty, rest)
                    } else {
                        Err(format!("{:?} cannot be accessed by {:?}", x, first))
                    }
                }
                _ => Err(format!("{:?} cannot be accessed by {:?}", x, first)),
            },
            Type::Dict(ty) => match first {
                Index::String(_) => recurse(ty, rest),
                _ => Err(format!("{:?} cannot be accessed by a non-string", cur_type)),
            },
            Type::Array(ty) => match first {
                Index::Number(_) => recurse(ty, rest),
                _ => Err(format!("{:?} cannot be accessed by a non-number", cur_type)),
            },
            Type::Union(_) => Err(format!("{:?} cannot be accessed by a non-number", cur_type)),
            Type::Intersection(_) => Err(format!("{:?} cannot be accessed by a non-number", cur_type)),
        }
    }

    recurse(cur_type, tail)
}
