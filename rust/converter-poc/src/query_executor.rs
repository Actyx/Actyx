//! Executes a full AQL query including filtering the FROM term
use std::{fmt::Display, str::FromStr};

use actyx_sdk::{
    language::{self, TagAtom, TagExpr},
    service::{EventResponse, PublishEvent},
    EventKey, Payload, TagSet,
};

#[derive(Debug)]
pub struct QueryExecutor {
    text: String,
    query: runtime::query::Query,
}

fn matches_atom(event: &EventResponse<Payload>, atom: &TagAtom) -> bool {
    match atom {
        TagAtom::AllEvents => true,
        TagAtom::Tag(tag) => event.tags.contains(tag),
        TagAtom::AppId(app_id) => app_id == &event.app_id,
        TagAtom::FromTime(time) => time <= &event.timestamp,
        TagAtom::ToTime(time) => &event.timestamp <= time,
        _ => todo!("not yet implemented!"),
    }
}

fn matches(event: &EventResponse<Payload>, from: &language::TagExpr) -> bool {
    match from {
        TagExpr::And(x) => matches(event, &x.0) && matches(event, &x.1),
        TagExpr::Or(x) => matches(event, &x.0) || matches(event, &x.1),
        TagExpr::Atom(atom) => matches_atom(event, atom),
    }
}

impl Display for QueryExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl FromStr for QueryExecutor {
    type Err = anyhow::Error;

    fn from_str(query: &str) -> Result<Self, Self::Err> {
        let text = query.to_owned();
        let query = query.parse::<language::Query>()?;
        Ok(Self {
            text,
            query: query.into(),
        })
    }
}

impl QueryExecutor {
    pub fn feed(&self, event: &EventResponse<Payload>, tags: &TagSet) -> Vec<PublishEvent> {
        let mut result = Vec::new();
        if matches(&event, &self.query.from) {
            let value = runtime::value::Value::from((
                EventKey {
                    lamport: event.lamport,
                    offset: event.offset,
                    stream: event.stream,
                },
                event.payload.clone(),
            ));
            for res in self.query.feed(value) {
                match res {
                    Ok(v) => {
                        result.push(PublishEvent {
                            tags: tags.clone(),
                            payload: v.payload(),
                        });
                    }
                    Err(e) => {
                        tracing::error!("OOPS {}", e);
                    }
                }
            }
        }
        result
    }
}
