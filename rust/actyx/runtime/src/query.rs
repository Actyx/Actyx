use crate::{
    eval::Context,
    operation::{Filter, Operation, Select},
    value::Value,
};
use actyx_sdk::language;

#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub features: Vec<String>,
    pub from: language::TagExpr,
    pub stages: Vec<Operation>,
}

impl Query {
    pub fn feed(&self, input: Value) -> Vec<Result<Value, String>> {
        fn rec<'a>(
            cx: &'a Context,
            input: Value,
            mut ops: impl Iterator<Item = &'a Operation> + Clone,
        ) -> Vec<Result<Value, String>> {
            if let Some(op) = ops.next() {
                let (vs, cx) = op.apply(cx, input);
                vs.into_iter()
                    .flat_map(|v| match v {
                        Ok(v) => rec(cx, v, ops.clone()),
                        Err(e) => vec![Err(e.to_string())],
                    })
                    .collect()
            } else {
                vec![Ok(input)]
            }
        }
        rec(&Context::new(input.key()), input, self.stages.iter())
    }
}

impl From<language::Query> for Query {
    fn from(q: language::Query) -> Self {
        let mut stages = vec![];
        let from = q.from;
        let features = q.features;
        for op in q.ops {
            match op {
                language::Operation::Filter(f) => stages.push(Operation::Filter(Filter::new(f))),
                language::Operation::Select(s) => stages.push(Operation::Select(Select::new(s.into_inner()))),
            }
        }
        Self { features, from, stages }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyx_sdk::{language::SortKey, NodeId};

    fn key() -> SortKey {
        SortKey {
            lamport: Default::default(),
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(0.into()),
        }
    }

    fn feed(q: &str, v: &str) -> Vec<String> {
        let q = Query::from(q.parse::<language::Query>().unwrap());
        let v = Context::new(key()).eval(&v.parse().unwrap()).unwrap();
        q.feed(v)
            .into_iter()
            .map(|v| v.map(|v| v.value().to_string()).unwrap_or_else(|e| e))
            .collect()
    }

    #[test]
    fn query() {
        assert_eq!(
            feed("FROM 'a' & isLocal FILTER _ < 3 SELECT _ + 2", "3"),
            Vec::<String>::new()
        );
        assert_eq!(feed("FROM 'a' & isLocal FILTER _ < 3 SELECT _ + 2", "2"), vec!["4"]);
    }

    #[test]
    fn select_multi() {
        assert_eq!(feed("FROM allEvents SELECT _, _ * 1.5", "42"), vec!["42", "63.0"]);
        assert_eq!(
            feed(
                "FROM allEvents SELECT _.x, _.y, _.z FILTER _ = 'a' SELECT _, 42",
                "{x:'a' y:'b'}"
            ),
            vec!["\"a\"", "42", r#"path .z does not exist in value {"x": "a", "y": "b"}"#]
        );
    }
}
