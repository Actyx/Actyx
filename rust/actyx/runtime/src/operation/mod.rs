use crate::{eval::Context, value::Value};
use actyx_sdk::{
    language::{self, NonEmptyVec, SimpleExpr},
    service::Order,
};

mod aggregate;
use futures::{future::BoxFuture, FutureExt};
use std::future::ready;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Operation {
    Filter(SimpleExpr),
    Select(NonEmptyVec<SimpleExpr>),
    Aggregate(SimpleExpr),
}

#[allow(unused_variables)]
pub(crate) trait Processor: Send + 'static {
    /// feed one value from upstream into this stage, bound to `_` in the Context
    fn apply<'a, 'b: 'a>(&'a mut self, cx: &'a Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>>;

    /// no more values will be fed, so possibly emit final outputs
    fn flush<'a, 'b: 'a>(&'a mut self, cx: &'a Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>> {
        ready(vec![]).boxed()
    }

    /// optimisation: declare whether a certain input value ordering would lead to being
    /// done earlier
    fn preferred_order(&self) -> Option<Order> {
        None
    }

    /// given that values have been and will be fed in `order`, will it make a difference
    /// to deliver further values?
    fn is_done(&self, order: Order) -> bool {
        false
    }
}

impl Operation {
    pub(super) fn make_processor(&self) -> Box<dyn Processor> {
        match self {
            Operation::Filter(f) => Box::new(Filter(f.clone())),
            Operation::Select(s) => Box::new(Select(s.clone())),
            Operation::Aggregate(a) => aggregate::aggregate(a),
        }
    }
}

impl From<language::Operation> for Operation {
    fn from(op: language::Operation) -> Self {
        match op {
            language::Operation::Filter(f) => Self::Filter(f),
            language::Operation::Select(s) => Self::Select(s),
            language::Operation::Aggregate(a) => Self::Aggregate(a),
        }
    }
}

struct Filter(SimpleExpr);
impl Processor for Filter {
    fn apply<'a, 'b: 'a>(&'a mut self, cx: &'a Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>> {
        async move {
            cx.eval(&self.0)
                .await
                .and_then(move |v| {
                    if v.as_bool()? {
                        Ok(Some(cx.lookup("_").unwrap().as_ref().unwrap().clone()))
                    } else {
                        Ok(None)
                    }
                })
                .transpose()
                .into_iter()
                .collect()
        }
        .boxed()
    }
}

struct Select(NonEmptyVec<SimpleExpr>);
impl Processor for Select {
    fn apply<'a, 'b: 'a>(&'a mut self, cx: &'a Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>> {
        async move {
            let mut v = vec![];
            for expr in self.0.iter() {
                v.push(cx.eval(expr).await)
            }
            v
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use actyx_sdk::{language::SortKey, NodeId, OffsetMap};
    use cbor_data::Encoder;

    use super::*;
    use std::convert::TryInto;
    use swarm::event_store_ref::EventStoreRef;

    fn simple_expr(s: &str) -> SimpleExpr {
        s.parse::<SimpleExpr>().unwrap()
    }

    fn key() -> SortKey {
        SortKey {
            lamport: Default::default(),
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(0.into()),
        }
    }
    fn store() -> EventStoreRef {
        EventStoreRef::new(|_x| Err(swarm::event_store_ref::Error::Aborted))
    }

    #[tokio::test]
    async fn filter() {
        let mut f = Filter(simple_expr("_ > 5 + a"));
        let mut cx = Context::owned(key(), Order::Asc, store(), OffsetMap::empty(), OffsetMap::empty());
        cx.bind("a", cx.value(|b| b.encode_f64(3.0)));

        cx.bind("_", cx.value(|b| b.encode_i64(8)));
        assert_eq!(f.apply(&cx).await.len(), 0);

        let v = cx.value(|b| b.encode_i64(9));
        cx.bind("_", v.clone());
        assert_eq!(f.apply(&cx).await.into_iter().next().unwrap().unwrap(), v);
    }

    #[tokio::test]
    async fn select() {
        let mut s = Select(vec![simple_expr("_.x + a")].try_into().unwrap());
        let mut cx = Context::owned(key(), Order::Asc, store(), OffsetMap::empty(), OffsetMap::empty());
        cx.bind("a", cx.value(|b| b.encode_f64(0.5)));

        cx.bind(
            "_",
            cx.value(|b| {
                b.encode_dict(|b| {
                    b.with_key("x", |b| b.encode_u64(2));
                })
            }),
        );
        assert_eq!(
            s.apply(&cx).await.into_iter().next().unwrap().unwrap(),
            cx.value(|b| b.encode_f64(2.5))
        );
    }
}
