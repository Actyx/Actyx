use super::{
    parse_utils::{Ps, Spanned},
    parser::{r_duration, r_simple_expr},
};
use crate::language::{
    parse_utils::{Ext, Span, P},
    parser::Rule,
    Ident, NonEmptyVec, SimpleExpr,
};
use ax_types::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Workflow<'a> {
    pub name: Ident,
    pub args: NonEmptyVec<Span<'a, Participant>>,
    pub steps: NonEmptyVec<WorkflowStep<'a>>,
}

impl<'a> Workflow<'a> {
    pub fn name(&self) -> &Ident {
        &self.name
    }

    pub fn args(&self) -> &NonEmptyVec<Span<'a, Participant>> {
        &self.args
    }

    pub fn steps(&self) -> &NonEmptyVec<WorkflowStep> {
        &self.steps
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Participant {
    Role(Ident),
    Unique(Ident),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WorkflowStep<'a> {
    Event {
        state: Option<Span<'a, Ident>>,
        mode: EventMode,
        label: Span<'a, Ident>,
        participant: Span<'a, Ident>,
        binders: Vec<Span<'a, Binding>>,
    },
    Retry {
        steps: NonEmptyVec<WorkflowStep<'a>>,
    },
    Timeout {
        micros: Span<'a, u64>,
        steps: NonEmptyVec<WorkflowStep<'a>>,
        mode: EventMode,
        label: Span<'a, Ident>,
        participant: Span<'a, Ident>,
        binders: Vec<Span<'a, Binding>>,
    },
    Parallel {
        count: Span<'a, u64>,
        cases: NonEmptyVec<NonEmptyVec<WorkflowStep<'a>>>,
    },
    Call {
        workflow: Span<'a, Ident>,
        args: NonEmptyVec<Span<'a, Ident>>,
        cases: NonEmptyVec<(Option<Span<'a, Ident>>, NonEmptyVec<WorkflowStep<'a>>)>,
    },
    Compensate {
        body: NonEmptyVec<WorkflowStep<'a>>,
        with: NonEmptyVec<WorkflowStep<'a>>,
    },
    Choice {
        cases: NonEmptyVec<NonEmptyVec<WorkflowStep<'a>>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventMode {
    Normal,
    Return,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Binding {
    pub name: Ident,
    pub role: Ident,
    pub value: SimpleExpr,
}

impl Binding {
    pub fn name(&self) -> &Ident {
        &self.name
    }

    pub fn role(&self) -> &Ident {
        &self.role
    }

    pub fn value(&self) -> &SimpleExpr {
        &self.value
    }
}

pub fn r_workflow(p: P) -> anyhow::Result<Workflow> {
    let mut p = p.into_inner();
    let name_span = p.peek().unwrap().as_span();
    let name = Ident(p.non_empty_string()?);
    let mut args = vec![];
    while matches!(p.next().unwrap().as_rule(), Rule::parenl | Rule::comma) {
        let p = p.next().unwrap();
        let span = p.as_span();
        let mut p = p.into_inner();
        let kind = p.next().unwrap();
        let participant = match kind.as_rule() {
            Rule::role => Participant::Role(Ident(p.non_empty_string()?)),
            Rule::unique => Participant::Unique(Ident(p.non_empty_string()?)),
            _ => unexpected!(kind),
        };
        args.push(Span::new(span, participant));
    }
    let steps = r_scope(p.next().unwrap())?;
    Ok(Workflow {
        name,
        args: NonEmptyVec::try_from(args).spanned(name_span)?,
        steps,
    })
}

fn r_scope(p: P) -> anyhow::Result<NonEmptyVec<WorkflowStep>> {
    let span = p.as_span();
    Ok(p.into_inner()
        .filter(|p| !matches!(p.as_rule(), Rule::curlyl | Rule::curlyr))
        .map(r_step)
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .spanned(span)?)
}

fn r_step(p: P) -> anyhow::Result<WorkflowStep> {
    Ok(match p.as_rule() {
        Rule::wf_event => r_event(p)?,
        Rule::wf_retry => WorkflowStep::Retry {
            steps: r_scope(p.into_inner().nth(1).unwrap())?,
        },
        Rule::wf_timeout => r_timeout(p)?,
        Rule::wf_parallel => {
            let mut p = p.into_inner();
            let parallel = p.next().unwrap();
            let count = if p.peek().map(|p| p.as_rule()) == Some(Rule::natural) {
                Some(Span::make(p.next().unwrap(), |mut p| p.natural())?)
            } else {
                None
            };
            let cases = r_cases(p)?;
            WorkflowStep::Parallel {
                count: count.unwrap_or_else(|| Span::new(parallel.as_span(), cases.len() as u64)),
                cases,
            }
        }
        Rule::wf_call => r_call(p)?,
        Rule::wf_compensate => {
            let mut p = p.into_inner();
            let _compensate = p.next().unwrap();
            let body = r_scope(p.next().unwrap())?;
            let _with = p.next().unwrap();
            let with = r_scope(p.next().unwrap())?;
            WorkflowStep::Compensate { body, with }
        }
        Rule::wf_choice => {
            let mut p = p.into_inner();
            let _choice = p.next().unwrap();
            let cases = r_cases(p)?;
            WorkflowStep::Choice { cases }
        }
        _ => unexpected!(p),
    })
}

fn r_event(p: P<'_>) -> Result<WorkflowStep<'_>, anyhow::Error> {
    let mut p = p.into_inner();
    let state = if p.peek().unwrap().as_rule() == Rule::wf_state {
        let mut ident = p.next().unwrap().single()?;
        let _colon = p.next().unwrap();
        Some(Span::new(ident.as_span(), Ident(ident.non_empty_string()?)))
    } else {
        None
    };
    let mode = match p.peek().unwrap().as_rule() {
        Rule::r#return => {
            p.next().unwrap();
            EventMode::Return
        }
        Rule::fail => {
            p.next().unwrap();
            EventMode::Fail
        }
        _ => EventMode::Normal,
    };
    let label = Span::make(p.next().unwrap(), |mut p| Ok(Ident(p.non_empty_string()?)))?;
    let _at = p.next().unwrap();
    let participant = Span::make(p.next().unwrap(), |mut p| Ok(Ident(p.non_empty_string()?)))?;
    let binders = p.map(r_binding).collect::<Result<Vec<_>, _>>()?;
    Ok(WorkflowStep::Event {
        state,
        mode,
        label,
        participant,
        binders,
    })
}

fn r_binding(p: P) -> anyhow::Result<Span<Binding>> {
    let span = p.as_span();
    let mut p = p.into_inner();
    let _curlyl = p.next().unwrap();
    let name = Ident(p.next().unwrap().non_empty_string()?);
    let _colon = p.next().unwrap();
    let role = Ident(p.next().unwrap().non_empty_string()?);
    let _arrowl = p.next().unwrap();
    let value = r_simple_expr(
        p.next().unwrap(),
        super::parser::Context::Simple { now: Timestamp::now() },
    )?;
    Ok(Span::new(span, Binding { name, role, value }))
}

fn r_timeout(p: P) -> anyhow::Result<WorkflowStep> {
    let mut p = p.into_inner();
    let _timeout = p.next().unwrap();
    let duration = Span::make(p.next().unwrap(), |p| r_duration(p))?;
    let steps = r_scope(p.next().unwrap())?;
    let WorkflowStep::Event {
        state: _,
        mode,
        label,
        participant,
        binders,
    } = r_event(p.next().unwrap())?
    else {
        unreachable!()
    };
    Ok(WorkflowStep::Timeout {
        micros: duration,
        steps,
        mode,
        label,
        participant,
        binders,
    })
}

fn r_cases(mut p: Ps) -> anyhow::Result<NonEmptyVec<NonEmptyVec<WorkflowStep>>> {
    let mut cases = vec![];
    let _curly = p.next().unwrap();
    let case = p.next().unwrap();
    while p.peek().is_some() {
        cases.push(
            (&mut p)
                .take_while(|p| !matches!(p.as_rule(), Rule::case | Rule::curlyr))
                .map(r_step)
                .collect::<Result<Vec<_>, _>>()?
                .try_into()
                .spanned(case.as_span())?,
        );
    }
    Ok(NonEmptyVec::try_from(cases).spanned(case.as_span())?)
}

fn r_call(p: P) -> anyhow::Result<WorkflowStep> {
    let mut p = p.into_inner();
    let _match = p.next().unwrap();
    let workflow = Span::make(p.next().unwrap(), |mut p| Ok(Ident(p.non_empty_string()?)))?;
    let paren = p.next().unwrap();
    let args = (&mut p)
        .take_while(|p| p.as_rule() != Rule::parenr)
        .filter(|p| p.as_rule() == Rule::ident)
        .map(|p| Span::make(p, |mut p| Ok(Ident(p.non_empty_string()?))))
        .collect::<Result<Vec<_>, _>>()?;
    let mut cases = vec![];
    let curly = p.next().unwrap();
    while let Some(p) = p.next() {
        if p.as_rule() == Rule::curlyr {
            break;
        }
        let mut p = p.into_inner();
        let _case = p.next().unwrap();
        let pat = p.next().unwrap();
        let label = if pat.as_rule() == Rule::ident {
            Some(Span::make(pat, |mut p| Ok(Ident(p.non_empty_string()?)))?)
        } else {
            None
        };
        let dblarrowr = p.next().unwrap();
        let steps = p.map(r_step).collect::<Result<Vec<_>, _>>()?;
        cases.push((label, NonEmptyVec::try_from(steps).spanned(dblarrowr.as_span())?));
    }
    Ok(WorkflowStep::Call {
        workflow,
        args: NonEmptyVec::try_from(args).spanned(paren.as_span())?,
        cases: NonEmptyVec::try_from(cases).spanned(curly.as_span())?,
    })
}

#[cfg(test)]
mod tests {
    use crate::Query;

    #[test]
    fn ex1() {
        let q = "WORKFLOW qg9gZK(UNIQUE kCSxum) { MATCH se5Q7Y(dJKW1Y) { CASE * => PARALLEL 15668459163393358498 { CASE exLLu3: ckZo9J @ BjfPBLR } } } FROM allEvents ORDER DESC END";
        Query::parse(q).unwrap();
    }
}
