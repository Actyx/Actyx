use crate::language::{parser::query_from_str, render::render_query, Query, StaticQuery};
use serde::{Deserialize, Serialize};

impl<'a> Query<'a> {
    pub fn parse(s: &'a str) -> anyhow::Result<Self> {
        query_from_str(s)
    }

    pub fn forget_pragmas(self) -> Query<'static> {
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
        }
    }

    pub fn decompose(self) -> (Query<'static>, Vec<String>, Vec<(&'a str, &'a str)>) {
        let features = self.features;
        let pragmas = self.pragmas;
        let q = Query {
            pragmas: vec![],
            features: vec![],
            ..self
        };
        (q, features, pragmas)
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
