use crate::{
    eval::Context,
    operation::{Operation, Processor},
    value::Value,
};
use actyx_sdk::{
    language::{self, Arr},
    service::Order,
};
use ax_futures_util::ReceiverExt;
use futures::{stream, StreamExt};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Query {
    pub features: Vec<String>,
    pub source: language::Source,
    pub stages: Vec<Operation>,
}

impl Query {
    pub fn from(q: language::Query<'_>) -> (Self, Vec<(&str, &str)>) {
        let pragmas = q.pragmas;
        let stages = q.ops.into_iter().map(Operation::from).collect();
        let source = q.source;
        let features = q.features;
        (
            Self {
                features,
                source,
                stages,
            },
            pragmas,
        )
    }

    /// run a query in the given evaluation context and collect all results
    pub async fn eval(query: &language::Query<'static>, cx: &Context<'_>) -> Result<Vec<Value>, anyhow::Error> {
        let mut feeder = Query::feeder_from(&query.ops);

        let (mut stream, cx) = match &query.source {
            language::Source::Events { from, order } => {
                let tag_expr = cx.eval_from(from).await?.into_owned();
                let (stream, cx) = if order.or_else(|| feeder.preferred_order()) == Some(Order::Desc) {
                    let stream = cx
                        .store
                        .bounded_backward(
                            tag_expr,
                            cx.from_offsets_excluding.as_ref().clone(),
                            cx.to_offsets_including.as_ref().clone(),
                        )
                        .await?
                        .stop_on_error();
                    (stream, cx.child_with_order(Order::Desc))
                } else {
                    let stream = cx
                        .store
                        .bounded_forward(
                            tag_expr,
                            cx.from_offsets_excluding.as_ref().clone(),
                            cx.to_offsets_including.as_ref().clone(),
                            false, // must keep order because some stage may have demanded it
                        )
                        .await?
                        .stop_on_error();
                    (stream, cx.child_with_order(Order::Asc))
                };
                let stream = stream
                    .map(|ev| match ev {
                        Ok(ev) => Ok(Value::from(ev)),
                        Err(e) => Err(e.into()),
                    })
                    .left_stream();
                (stream, cx)
            }
            language::Source::Array(Arr { items }) => (
                stream::iter(items.iter()).then(|expr| cx.eval(expr)).right_stream(),
                cx.child(),
            ),
        };

        let mut results = vec![];
        while let Some(ev) = stream.next().await {
            let value = ev?;
            let vs = feeder.feed(Some(value), &cx).await;
            results.reserve(vs.len());
            for v in vs {
                results.push(v?);
            }
            if feeder.is_done() {
                break;
            }
        }
        drop(stream);

        let vs = feeder.feed(None, &cx).await;
        results.reserve(vs.len());
        for v in vs {
            results.push(v?);
        }
        Ok(results)
    }

    pub fn make_feeder(&self) -> Feeder {
        let processors = self.stages.iter().map(|op| op.make_processor()).collect();
        Feeder::new(processors)
    }

    pub fn feeder_from(stages: &[language::Operation]) -> Feeder {
        let processors = stages
            .iter()
            .map(|s| Operation::from(s.clone()).make_processor())
            .collect();
        Feeder::new(processors)
    }
}

pub struct Feeder {
    is_done: bool,
    processors: Vec<Box<dyn Processor>>,
}
impl Feeder {
    fn new(processors: Vec<Box<dyn Processor>>) -> Self {
        Self {
            is_done: false,
            processors,
        }
    }

    pub fn preferred_order(&self) -> Option<Order> {
        for op in &self.processors {
            if let Some(order) = op.preferred_order() {
                return Some(order);
            }
        }
        None
    }

    pub fn is_done(&self) -> bool {
        self.is_done
    }

    pub async fn feed(&mut self, input: Option<Value>, cx: &Context<'_>) -> Vec<Result<Value, anyhow::Error>> {
        // create storage space for one context per stage
        let mut ctx = Vec::<Option<Context<'_>>>::new();
        ctx.resize_with(self.processors.len(), || None);
        let mut ctx = &mut ctx[..];

        // create the outermost context, stored on the stack
        let mut cx = cx.child();

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
            if op.is_done(cx.order) {
                self.is_done = true;
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

#[cfg(test)]
mod tests {
    use super::*;
    use actyx_sdk::OffsetMap;
    use swarm::event_store_ref::EventStoreRef;

    fn store() -> EventStoreRef {
        EventStoreRef::new(|_x| Err(swarm::event_store_ref::Error::Aborted))
    }
    fn ctx(order: Order) -> Context<'static> {
        Context::owned(order, store(), OffsetMap::empty(), OffsetMap::empty())
    }
    fn feeder(q: &str) -> Feeder {
        Query::from(language::Query::parse(q).unwrap()).0.make_feeder()
    }

    async fn feed(q: &str, v: &str) -> Vec<String> {
        let cx = ctx(Order::Asc);
        let v = cx.eval(&v.parse().unwrap()).await.unwrap();
        feeder(q)
            .feed(Some(v), &cx)
            .await
            .into_iter()
            .map(|v| v.map(|v| v.cbor().to_string()).unwrap_or_else(|e| e.to_string()))
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
            vec!["\"a\"", "42", r#"property `z` not found in Object"#]
        );
    }

    #[test]
    fn order() {
        assert_eq!(feeder("FROM 'x' SELECT y").preferred_order(), None);
        assert_eq!(feeder("FROM 'x' FILTER x").preferred_order(), None);
        assert_eq!(feeder("FROM 'x' AGGREGATE x").preferred_order(), None);
        assert_eq!(
            feeder("FROM 'x' AGGREGATE LAST(_)").preferred_order(),
            Some(Order::Desc)
        );
        assert_eq!(
            feeder("FROM 'x' AGGREGATE FIRST(_)").preferred_order(),
            Some(Order::Asc)
        );
        assert_eq!(feeder("FROM 'x' AGGREGATE [FIRST(_), LAST(_)]").preferred_order(), None);
        assert_eq!(feeder("FROM 'x' AGGREGATE SUM(_)").preferred_order(), None);
    }

    #[tokio::test]
    async fn done() {
        let mut f = feeder("FROM 'x' AGGREGATE x");
        let cx = ctx(Order::Asc);
        let v = cx.eval(&"42".parse().unwrap()).await.unwrap();
        f.feed(Some(v), &cx).await;
        assert!(f.is_done());

        let mut f = feeder("FROM 'x' AGGREGATE x");
        let cx = ctx(Order::Desc);
        let v = cx.eval(&"42".parse().unwrap()).await.unwrap();
        f.feed(Some(v), &cx).await;
        assert!(f.is_done());

        let mut f = feeder("FROM 'x' AGGREGATE LAST(_)");
        let cx = ctx(Order::Asc);
        let v = cx.eval(&"42".parse().unwrap()).await.unwrap();
        f.feed(Some(v), &cx).await;
        assert!(!f.is_done());

        let mut f = feeder("FROM 'x' AGGREGATE LAST(_)");
        let cx = ctx(Order::Desc);
        let v = cx.eval(&"42".parse().unwrap()).await.unwrap();
        f.feed(Some(v), &cx).await;
        assert!(f.is_done());

        let mut f = feeder("FROM 'x' AGGREGATE FIRST(_)");
        let cx = ctx(Order::Asc);
        let v = cx.eval(&"42".parse().unwrap()).await.unwrap();
        f.feed(Some(v), &cx).await;
        assert!(f.is_done());

        let mut f = feeder("FROM 'x' AGGREGATE FIRST(_)");
        let cx = ctx(Order::Desc);
        let v = cx.eval(&"42".parse().unwrap()).await.unwrap();
        f.feed(Some(v), &cx).await;
        assert!(!f.is_done());
    }

    #[tokio::test]
    async fn binding() {
        assert_eq!(
            feed("FROM 'x' LET x := _.a SELECT [_.b, x]", "{a:1 b:2 c:3}").await,
            vec!["[2, 1]"]
        );
    }
}
