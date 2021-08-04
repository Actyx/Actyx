use crate::{
    eval::Context,
    operation::{Aggregate, Filter, Operation, Select},
    value::Value,
};
use actyx_sdk::language;

#[derive(Debug, PartialEq)]
pub struct Query {
    pub features: Vec<String>,
    pub from: language::TagExpr,
    pub stages: Vec<Operation>,
}

impl Query {
    pub fn feed(&mut self, input: Value) -> Vec<Result<Value, String>> {
        fn rec(stages: &mut Vec<Operation>, current: usize, cx: &Context, input: Value) -> Vec<Result<Value, String>> {
            if let Some(op) = stages.get_mut(current) {
                let vs = op.apply(cx, input);
                vs.into_iter()
                    .flat_map(|v| match v {
                        Ok(v) => rec(stages, current + 1, cx, v),
                        Err(e) => vec![Err(e.to_string())],
                    })
                    .collect()
            } else {
                vec![Ok(input)]
            }
        }
        rec(&mut self.stages, 0, &Context::new(input.key()), input)
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
                language::Operation::Select(s) => stages.push(Operation::Select(Select::new(s))),
                language::Operation::Aggregate(a) => stages.push(Operation::Aggregate(Aggregate::new(a))),
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
        let mut q = Query::from(q.parse::<language::Query>().unwrap());
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
