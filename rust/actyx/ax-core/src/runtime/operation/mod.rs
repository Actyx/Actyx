use crate::runtime::{
    error::{RuntimeError, RuntimeFailure},
    eval::Context,
    query::Query,
    value::Value,
};
use ax_aql::{NonEmptyVec, SimpleExpr, SpreadExpr};
use ax_types::service::Order;

mod aggregate;
use futures::{future::BoxFuture, FutureExt};
use std::{future::ready, num::NonZeroU64};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Operation {
    Filter(SimpleExpr),
    Select(NonEmptyVec<SpreadExpr>),
    Aggregate(SimpleExpr),
    Limit(NonZeroU64),
    Binding(String, SimpleExpr),
}

#[allow(unused_variables)]
pub(crate) trait Processor: Send + 'static {
    /// feed one value from upstream into this stage, bound to `_` in the Context
    fn apply<'a, 'b: 'a>(&'a mut self, cx: &'a mut Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>>;

    /// no more values will be fed, so possibly emit final outputs
    fn flush<'a, 'b: 'a>(&'a mut self, cx: &'a mut Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>> {
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
            Operation::Limit(l) => Box::new(Limit((*l).into())),
            Operation::Binding(n, e) => Box::new(Binding(n.clone(), e.clone())),
        }
    }
}

impl From<ax_aql::Operation> for Operation {
    fn from(op: ax_aql::Operation) -> Self {
        match op {
            ax_aql::Operation::Filter(f) => Self::Filter(f),
            ax_aql::Operation::Select(s) => Self::Select(s),
            ax_aql::Operation::Aggregate(a) => Self::Aggregate(a),
            ax_aql::Operation::Limit(l) => Self::Limit(l),
            ax_aql::Operation::Binding(n, e) => Self::Binding(n, e),
        }
    }
}

struct Filter(SimpleExpr);
impl Processor for Filter {
    fn apply<'a, 'b: 'a>(&'a mut self, cx: &'a mut Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>> {
        async move {
            /*
             * Anti-flag propagation:
             *
             * This is automatic in this case because the value bound to "_" has the
             * flag and we take it out of the context and hand it back.
             */
            cx.eval(&self.0)
                .await
                .and_then(move |v| {
                    if v.as_bool()? {
                        Ok(Some(cx.remove("_")?))
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

struct Select(NonEmptyVec<SpreadExpr>);
impl Processor for Select {
    fn apply<'a, 'b: 'a>(&'a mut self, cx: &'a mut Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>> {
        async move {
            let mut v = vec![];
            /*
             * Anti-flag propagation:
             *
             * The idea here is that an anti-input produces anti-outputs. Instead of
             * tracking this throughout the evaluation (with some difficulty regarding
             * inputs stemming from different inputs) we slap the anti-flag on all
             * outputs in the end.
             */
            let anti = cx
                .lookup_opt("_")
                .map(|v| v.as_ref().map(|v| v.is_anti()).unwrap_or_default())
                .unwrap_or_default();
            for expr in self.0.iter() {
                if let (SimpleExpr::SubQuery(e), true) = (&expr.expr, expr.spread) {
                    match Query::eval(e, cx).await {
                        Ok(arr) => v.extend(arr.into_iter().map(Ok)),
                        Err(e) => v.push(Err(e)),
                    }
                } else {
                    match cx.eval(expr).await {
                        Ok(val) => {
                            if expr.spread {
                                if let Ok(items) = val.as_array() {
                                    v.extend(items.into_iter().map(Ok));
                                } else {
                                    v.push(Err(RuntimeError::TypeErrorSpread(val.kind()).into()))
                                }
                            } else {
                                v.push(Ok(val));
                            }
                        }
                        Err(e) => v.push(Err(e)),
                    }
                }
            }
            if anti {
                for v in v.iter_mut().flatten() {
                    v.anti();
                }
            }
            v
        }
        .boxed()
    }
}

struct Limit(u64);
impl Processor for Limit {
    fn apply<'a, 'b: 'a>(&'a mut self, cx: &'a mut Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>> {
        async move {
            /*
             * Anti-flag propagation:
             *
             * As long as the limit has not been exhausted, anti-inputs become anti-outputs;
             * the latter increment the limit again. As soon as the limit has reached zero,
             * anti-input cannot meaningfully be processed since we cannot know whether the
             * corresponding output was emitted or suppressed earlier, so we stop the query
             * with an error.
             */
            if self.0 > 0 {
                let v = cx.remove("_");
                match &v {
                    Ok(v) if v.is_anti() => self.0 += 1,
                    Ok(_) => self.0 -= 1,
                    _ => {}
                }
                vec![v]
            } else {
                let anti = cx.remove("_").map(|v| v.is_anti()).unwrap_or_default();
                if anti {
                    vec![Err(RuntimeFailure::AntiInputInLimit.into())]
                } else {
                    vec![]
                }
            }
        }
        .boxed()
    }

    fn is_done(&self, _order: Order) -> bool {
        self.0 == 0
    }
}

struct Binding(String, SimpleExpr);
impl Processor for Binding {
    fn apply<'a, 'b: 'a>(&'a mut self, cx: &'a mut Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>> {
        async move {
            /*
             * Anti-flag propagation:
             *
             * This is automatic in this case because the value bound to "_" has the
             * flag and we take it out of the context and hand it back.
             */
            match cx.eval(&self.1).await {
                Ok(v) => {
                    cx.bind(self.0.as_str(), v);
                    vec![cx.remove("_")]
                }
                Err(e) => vec![Err(e)],
            }
        }
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swarm::event_store_ref::EventStoreRef;
    use cbor_data::Encoder;
    use std::convert::TryInto;

    fn simple_expr(s: &str) -> SimpleExpr {
        s.parse::<SimpleExpr>().unwrap()
    }

    fn store() -> EventStoreRef {
        EventStoreRef::new(|_x| Err(crate::swarm::event_store_ref::Error::Aborted))
    }

    #[tokio::test]
    async fn filter() {
        let mut f = Filter(simple_expr("_ > 5 + a"));
        let cx = Context::new(store());
        let mut cx = cx.child();
        cx.bind("a", Value::synthetic(cx.mk_cbor(|b| b.encode_f64(3.0))));

        cx.bind("_", Value::synthetic(cx.mk_cbor(|b| b.encode_i64(8))));
        assert_eq!(f.apply(&mut cx).await.len(), 0);

        let v = Value::synthetic(cx.mk_cbor(|b| b.encode_i64(9)));
        cx.bind("_", v.clone());
        assert_eq!(f.apply(&mut cx).await.into_iter().next().unwrap().unwrap(), v);
    }

    #[tokio::test]
    async fn select() {
        let mut s = Select(vec![simple_expr("_.x + a").with_spread(false)].try_into().unwrap());
        let cx = Context::new(store());
        let mut cx = cx.child();
        cx.bind("a", Value::synthetic(cx.mk_cbor(|b| b.encode_f64(0.5))));

        cx.bind(
            "_",
            Value::synthetic(cx.mk_cbor(|b| {
                b.encode_dict(|b| {
                    b.with_key("x", |b| b.encode_u64(2));
                })
            })),
        );
        assert_eq!(
            s.apply(&mut cx).await.into_iter().next().unwrap().unwrap(),
            Value::synthetic(cx.mk_cbor(|b| b.encode_f64(2.5)))
        );
    }
}
