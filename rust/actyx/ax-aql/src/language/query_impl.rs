use super::workflow::Workflow;
use crate::{
    language::{parser::query_from_str, render::render_query, Query, StaticQuery},
    Ident, Label, Type,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Deref, sync::Arc};

impl<'a> Query<'a> {
    pub fn parse(s: &'a str) -> anyhow::Result<Self> {
        query_from_str(s)
    }

    pub fn forget_pragmas_and_workflows(self) -> Query<'static> {
        let features = self.features;
        let source = self.source;
        let ops = self.ops;
        let events = self.events;
        Query {
            pragmas: Vec::new(),
            features,
            source,
            ops,
            events,
            workflows: Arc::new(BTreeMap::new()),
        }
    }

    pub fn decompose(
        self,
    ) -> (
        Query<'static>,
        Vec<String>,
        Vec<(&'a str, &'a str)>,
        Arc<BTreeMap<Ident, Workflow<'a>>>,
    ) {
        let features = self.features;
        let pragmas = self.pragmas;
        let q = Query {
            pragmas: vec![],
            features: vec![],
            workflows: Arc::new(BTreeMap::new()),
            ..self
        };
        (q, features, pragmas, self.workflows)
    }

    pub fn get_used_event_types(&'a self) -> impl Iterator<Item = (Label, Type)> + 'a {
        self.workflows
            .iter()
            .flat_map(|(_, workflow)| workflow.steps.iter().map(|step| step.get_events()))
            .flatten()
            // if we just took the idents earlier we could probably replace the vecs in get_events with iterators
            .map(|events| events.deref().clone())
            .filter_map(|ident| self.events.get(&ident).map(|(ty, _)| (Label::from(ident), ty.clone())))
    }
}

impl<'de> Deserialize<'de> for Query<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        query_from_str(s).map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for StaticQuery {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let q = Query::deserialize(deserializer)?;
        Ok(StaticQuery(Query {
            pragmas: Vec::new(),
            features: q.features,
            source: q.source,
            ops: q.ops,
            events: q.events,
            workflows: Arc::new(BTreeMap::new()),
        }))
    }
}

impl<'a> Serialize for Query<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'a> std::fmt::Display for Query<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        render_query(f, self)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::{Label, Query};

    fn slice_to_labels(s: &[&str]) -> HashSet<Label> {
        s.iter().map(|s| Label::try_from(*s).unwrap()).collect()
    }

    #[test]
    fn test_used_events_empty() {
        let query = Query::parse(
            "
            EVENT start { _: NULL }
            FROM allEvents
            ",
        )
        .expect("should be a valid query");

        let event_types = query.get_used_event_types().collect::<HashSet<_>>();
        assert!(event_types.is_empty());
    }

    #[test]
    fn test_used_events_event_step() {
        let query = Query::parse(
            "
            EVENT start { _: NULL }
            EVENT done  { _: NULL }

            WORKFLOW w (UNIQUE a) {
                start @ a
            }

            FROM allEvents
            ",
        )
        .expect("should be a valid query");

        let labels = query.get_used_event_types().map(|t| t.0).collect::<HashSet<_>>();
        assert!(!labels.is_empty());
        assert_eq!(labels, slice_to_labels(&["start"]));
    }

    #[test]
    fn test_used_events_retry_step() {
        let query = Query::parse(
            "
            EVENT start { _: NULL }
            EVENT done  { _: NULL }

            WORKFLOW w (UNIQUE a) {
                RETRY {
                    start @ a
                }
            }

            FROM allEvents
            ",
        )
        .expect("should be a valid query");

        let labels = query.get_used_event_types().map(|t| t.0).collect::<HashSet<_>>();
        assert!(!labels.is_empty());
        assert_eq!(labels, slice_to_labels(&["start"]));
    }

    #[test]
    fn test_used_events_timeout_step() {
        let query = Query::parse(
            "
            EVENT start   { _: NULL }
            EVENT timeout { _: NULL }
            EVENT done    { _: NULL }

            WORKFLOW w (UNIQUE a) {
                TIMEOUT 1m {
                    start @ a
                } RETURN timeout @ a
            }

            FROM allEvents
            ",
        )
        .expect("should be a valid query");

        let labels = query.get_used_event_types().map(|t| t.0).collect::<HashSet<_>>();
        assert!(!labels.is_empty());
        assert_eq!(labels, slice_to_labels(&["start", "timeout"]));
    }

    #[test]
    fn test_used_events_parallel_step() {
        let query = Query::parse(
            "
            EVENT start { _: NULL }
            EVENT done  { _: NULL }

            WORKFLOW w (UNIQUE a) {
                PARALLEL 2 {
                    CASE start @ a
                }
            }

            FROM allEvents
            ",
        )
        .expect("should be a valid query");

        let labels = query.get_used_event_types().map(|t| t.0).collect::<HashSet<_>>();
        assert!(!labels.is_empty());
        assert_eq!(labels, slice_to_labels(&["start"]));
    }

    #[test]
    fn test_used_events_call_step() {
        let query = Query::parse(
            "
            EVENT start { _: NULL }
            EVENT done  { _: NULL }

            WORKFLOW callee (UNIQUE a) {
                start @ a
            }

            WORKFLOW caller (UNIQUE a) {
                MATCH callee (a) {
                    CASE * => start @ a
                }
            }

            FROM allEvents
            ",
        )
        .expect("should be a valid query");

        let labels = query.get_used_event_types().map(|t| t.0).collect::<HashSet<_>>();
        assert!(!labels.is_empty());
        assert_eq!(labels, slice_to_labels(&["start"]));
    }

    #[test]
    fn test_used_events_compensate_step() {
        let query = Query::parse(
            "
            EVENT start { _: NULL }
            EVENT done  { _: NULL }

            WORKFLOW caller (UNIQUE a) {
                COMPENSATE {
                    start @ a
                } WITH {
                    -- this workflow doesn't make sense
                    -- but we're not testing semantics
                    start @ a
                }
            }

            FROM allEvents
            ",
        )
        .expect("should be a valid query");

        let labels = query.get_used_event_types().map(|t| t.0).collect::<HashSet<_>>();
        assert!(!labels.is_empty());
        assert_eq!(labels, slice_to_labels(&["start"]));
    }

    #[test]
    fn test_used_events_choice_step() {
        let query = Query::parse(
            "
            EVENT start { _: NULL }
            EVENT pause { _: NULL }
            EVENT done  { _: NULL }

            WORKFLOW caller (UNIQUE a) {
                CHOICE {
                    CASE start @ a
                    CASE pause @ a
                }
            }

            FROM allEvents
            ",
        )
        .expect("should be a valid query");

        let labels = query.get_used_event_types().map(|t| t.0).collect::<HashSet<_>>();
        assert!(!labels.is_empty());
        assert_eq!(labels, slice_to_labels(&["start", "pause"]));
    }

    #[test]
    fn test_used_events_retry_timeout_choice_match_parallel_steps() {
        let query = Query::parse(
            "
            EVENT start   { _: NULL }
            EVENT pause   { _: NULL }
            EVENT timeout { _: NULL }
            EVENT done    { _: NULL }

            WORKFLOW callee (UNIQUE a) {
                PARALLEL 1 {
                    CASE start @ a
                }
            }

            WORKFLOW caller (UNIQUE a) {
                RETRY {
                    TIMEOUT 1m {
                        CHOICE {
                            CASE start @ a
                        }
                    } RETURN timeout @ a
                }
                MATCH callee(a) {
                    CASE * => pause @ a
                }
            }

            FROM allEvents
            ",
        )
        .expect("should be a valid query");

        let labels = query.get_used_event_types().map(|t| t.0).collect::<HashSet<_>>();
        assert!(!labels.is_empty());
        assert_eq!(labels, slice_to_labels(&["start", "pause", "timeout"]));
    }
}
