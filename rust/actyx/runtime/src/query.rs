use crate::{
    eval::Context,
    operation::{Operation, Processor},
    value::Value,
};
use actyx_sdk::language;
use ax_futures_util::ReceiverExt;
use cbor_data::{Encoder, Writer};
use futures::StreamExt;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Query {
    pub features: Vec<String>,
    pub from: language::TagExpr,
    pub stages: Vec<Operation>,
}

impl Query {
    /// run a query in the given evaluation context and collect all results
    pub async fn eval(query: &language::Query, cx: &Context<'_>) -> Result<Value, anyhow::Error> {
        let mut feeder = Query::feeder_from(&query.ops);
        let mut stream = cx
            .store
            .bounded_forward(
                query.from.clone(),
                cx.from_offsets_excluding.as_ref().clone(),
                cx.to_offsets_including.as_ref().clone(),
                true,
            )
            .await?
            .stop_on_error();

        let mut results = vec![];
        while let Some(ev) = stream.next().await {
            let value = Value::from((ev.key, ev.payload.clone()));
            let vs = feeder.feed(Some(value), cx).await;
            results.reserve(vs.len());
            for v in vs {
                results.push(v?);
            }
        }

        let vs = feeder.feed(None, cx).await;
        results.reserve(vs.len());
        for v in vs {
            results.push(v?);
        }
        Ok(cx.value(move |b| {
            b.encode_array(move |b| {
                for v in results.drain(..) {
                    b.write_trusting(v.as_slice());
                }
            })
        }))
    }

    pub fn make_feeder(&self) -> Feeder {
        let processors = self.stages.iter().map(|op| op.make_processor()).collect();
        Feeder { processors }
    }

    pub fn feeder_from(stages: &[language::Operation]) -> Feeder {
        let processors = stages
            .iter()
            .map(|s| Operation::from(s.clone()).make_processor())
            .collect();
        Feeder { processors }
    }
}

pub struct Feeder {
    processors: Vec<Box<dyn Processor>>,
}
impl Feeder {
    pub async fn feed(&mut self, input: Option<Value>, cx: &Context<'_>) -> Vec<Result<Value, anyhow::Error>> {
        // create storage space for one context per stage
        let mut ctx = Vec::<Option<Context<'_>>>::new();
        ctx.resize_with(self.processors.len(), || None);
        let mut ctx = &mut ctx[..];

        // create the outermost context, stored on the stack
        let mut cx = cx.child();
        if let Some(ref v) = input {
            cx.sort_key = v.key();
        }

        // set up per-iteration state
        let mut cx = &mut cx; // reference to the current context
        let mut input = vec![Ok(input).transpose()]; // inputs to be delivered to the current stage

        for op in self.processors.iter_mut() {
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
                        output.extend(op.apply(cx).await.into_iter().map(Some));
                    }
                    None => {
                        output.extend(op.flush(cx).await.into_iter().map(Some));
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

        // get rid of the Option wrapper and the possibly trailing None
        input.into_iter().flatten().collect()
    }
}

impl From<language::Query> for Query {
    fn from(q: language::Query) -> Self {
        let stages = q.ops.into_iter().map(Operation::from).collect();
        let from = q.from;
        let features = q.features;
        Self { features, from, stages }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyx_sdk::{language::SortKey, NodeId, OffsetMap};
    use swarm::event_store_ref::EventStoreRef;

    fn key() -> SortKey {
        SortKey {
            lamport: Default::default(),
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(0.into()),
        }
    }
    fn store() -> EventStoreRef {
        EventStoreRef::new(|_x| Err(swarm::event_store_ref::Error::Aborted))
    }

    async fn feed(q: &str, v: &str) -> Vec<String> {
        let q = Query::from(q.parse::<language::Query>().unwrap());
        let cx = Context::owned(key(), store(), OffsetMap::empty(), OffsetMap::empty());
        let v = cx.eval(&v.parse().unwrap()).await.unwrap();
        q.make_feeder()
            .feed(Some(v), &cx)
            .await
            .into_iter()
            .map(|v| v.map(|v| v.value().to_string()).unwrap_or_else(|e| e.to_string()))
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
