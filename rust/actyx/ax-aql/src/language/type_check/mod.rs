use super::workflow::{Participant, Workflow, WorkflowStep};
use crate::{Ident, NonEmptyVec, Query, SimpleExpr};
use std::collections::BTreeSet;

mod drill_down_type;

pub(crate) struct QueryTypeCheckError<'a> {
    pub(crate) span: pest::Span<'a>,
    pub(crate) error_type: QueryTypeCheckErrorType,
}

pub(crate) enum QueryTypeCheckErrorType {
    UndeclaredEvent,
    UndeclaredParticipant,
    UndeclaredWorkflowCall,
    InvalidIndex(String),
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

pub(crate) fn check<'a>(query: &Query<'a>) -> Vec<QueryTypeCheckError<'a>> {
    query
        .workflows
        .iter()
        .flat_map(|(_ident, workflow)| check_workflow(query, workflow))
        .collect()
}

fn check_workflow<'a>(query: &Query<'a>, workflow: &Workflow<'a>) -> Vec<QueryTypeCheckError<'a>> {
    check_steps(query, &WorkflowAnalysisResource::from(workflow), &workflow.steps)
}

fn check_steps<'a>(
    query: &Query<'a>,
    parent_workflow: &WorkflowAnalysisResource,
    steps: &NonEmptyVec<WorkflowStep<'a>>,
) -> Vec<QueryTypeCheckError<'a>> {
    steps
        .iter()
        .flat_map(|step| check_step(query, parent_workflow, step))
        .collect()
}

fn check_step<'a>(
    query: &Query<'a>,
    parent_workflow: &WorkflowAnalysisResource,
    step: &WorkflowStep<'a>,
) -> Vec<QueryTypeCheckError<'a>> {
    use super::workflow::WorkflowStep as W;

    match step {
        W::Event { label, binders, .. } => {
            let mut errors = Vec::new();
            let matching_event = query.events.get(label);

            if matching_event.is_none() {
                errors.push(QueryTypeCheckError {
                    error_type: QueryTypeCheckErrorType::UndeclaredEvent,
                    span: label.span(),
                });
            }

            if let Some((ev_type, _tag)) = matching_event {
                binders.iter().for_each(|binding| {
                    let value = binding.value();

                    let res: Result<(), String> = match value {
                        SimpleExpr::Indexing(ind) => {
                            drill_down_type::drill_down_type_by_index(ev_type, ind).and(Ok(()))
                        }
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
        W::Retry { steps } => check_steps(query, parent_workflow, steps),
        W::Timeout { steps, .. } => check_steps(query, parent_workflow, steps),
        W::Parallel { cases, .. } => cases
            .iter()
            .flat_map(|case| check_steps(query, parent_workflow, case))
            .collect(),
        W::Call {
            workflow: workflow_ident,
            cases,
            ..
        } => {
            let mut errors = Vec::new();
            if query.workflows.get(workflow_ident).is_none() {
                errors.push(QueryTypeCheckError {
                    error_type: QueryTypeCheckErrorType::UndeclaredWorkflowCall,
                    span: workflow_ident.span(),
                });
            }

            errors.extend(
                cases
                    .iter()
                    .flat_map(|(_, steps)| check_steps(query, parent_workflow, steps)),
            );

            errors
        }
        W::Compensate { body, with } => std::iter::empty()
            .chain(check_steps(query, parent_workflow, body))
            .chain(check_steps(query, parent_workflow, with))
            .collect(),
        W::Choice { cases } => cases
            .iter()
            .flat_map(|steps| check_steps(query, parent_workflow, steps))
            .collect(),
    }
}
