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
    /// Feed a new value into this processing pipeline, or feed None to flush aggregations.
    pub async fn feed(&mut self, input: Option<Value>) -> Vec<Result<Value, String>> {
        // create storage space for one context per stage
        let mut ctx = Vec::<Option<Context<'_>>>::new();
        ctx.resize_with(self.stages.len(), || None);
        let mut ctx = &mut ctx[..];

        let mut cx = Context::new(input.as_ref().map(|x| x.key()).unwrap_or_default());

        // set up per-iteration state
        let mut cx = &mut cx;
        let mut input = vec![Ok(input).transpose()];

        fn adapt(r: anyhow::Result<Value>) -> Option<Result<Value, String>> {
            Some(r.map_err(|e| e.to_string()))
        }

        for op in self.stages.iter_mut() {
            // create fresh child context, stored in the ctx slice
            let (curr_ctx, rest) = ctx.split_first_mut().unwrap();
            ctx = rest;
            *curr_ctx = Some(cx.child());
            cx = curr_ctx.as_mut().unwrap();
            // then feed all inputs
            let mut output = vec![];
            for input in input {
                match input {
                    Some(Ok(v)) => {
                        cx.bind("_", v);
                        output.extend(op.apply(cx).await.into_iter().map(adapt));
                    }
                    None => {
                        output.extend(op.flush(cx).await.into_iter().map(adapt));
                        output.push(None);
                    }
                    Some(Err(e)) => output.push(Some(Err(e))),
                }
            }
            input = output;
            if input.is_empty() {
                break;
            }
        }

        input.into_iter().flatten().collect()
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

    async fn feed(q: &str, v: &str) -> Vec<String> {
        let mut q = Query::from(q.parse::<language::Query>().unwrap());
        let v = Context::new(key()).eval(&v.parse().unwrap()).await.unwrap();
        q.feed(Some(v))
            .await
            .into_iter()
            .map(|v| v.map(|v| v.value().to_string()).unwrap_or_else(|e| e))
            .collect()
    }

    #[tokio::test]
    async fn query() {
        assert_eq!(
            feed("FROM 'a' & isLocal FILTER _ < 3 SELECT _ + 2", "3").await,
            Vec::<String>::new()
        );
        assert_eq!(
            feed("FROM 'a' & isLocal FILTER _ < 3 SELECT _ + 2", "2").await,
            vec!["4"]
        );
    }

    #[tokio::test]
    async fn select_multi() {
        assert_eq!(feed("FROM allEvents SELECT _, _ * 1.5", "42").await, vec!["42", "63.0"]);
        assert_eq!(
            feed(
                "FROM allEvents SELECT _.x, _.y, _.z FILTER _ = 'a' SELECT _, 42",
                "{x:'a' y:'b'}"
            )
            .await,
            vec!["\"a\"", "42", r#"path .z does not exist in value {"x": "a", "y": "b"}"#]
        );
    }
}
