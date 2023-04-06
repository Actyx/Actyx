use crate::{
    error::{RuntimeError, RuntimeFailure},
    eval::Context,
    value::Value,
};
use actyx_sdk::{
    language::{AggrOp, Galactus, Num, SimpleExpr, Tactic, Var},
    service::{EventMeta, Order},
};
use anyhow::anyhow;
use cbor_data::Encoder;
use futures::{future::BoxFuture, FutureExt};
use std::{cmp::Ordering, marker::PhantomData, ops::AddAssign, sync::Arc};

pub trait Aggregator {
    fn feed(&mut self, input: Value) -> anyhow::Result<()>;
    fn flush(&mut self, cx: &Context) -> anyhow::Result<Value>;
    fn has_value(&self) -> bool;
}

trait SumOp {
    fn bool(l: u64, r: bool, anti: bool) -> u64;
    fn to_bool(n: u64) -> bool;
    fn num(l: Num, r: Num, anti: bool) -> anyhow::Result<Num>;
}
#[derive(Default)]
struct AddOp;
impl SumOp for AddOp {
    fn bool(l: u64, r: bool, anti: bool) -> u64 {
        if r {
            if anti {
                l - 1
            } else {
                l + 1
            }
        } else {
            l
        }
    }
    fn to_bool(n: u64) -> bool {
        n > 0
    }
    fn num(l: Num, r: Num, anti: bool) -> anyhow::Result<Num> {
        if anti {
            Ok(l.sub(&r)?)
        } else {
            Ok(l.add(&r)?)
        }
    }
}
#[derive(Default)]
struct MulOp;
impl SumOp for MulOp {
    fn bool(l: u64, r: bool, anti: bool) -> u64 {
        if r {
            l
        } else if anti {
            l - 1
        } else {
            l + 1
        }
    }
    fn to_bool(n: u64) -> bool {
        n == 0
    }
    fn num(l: Num, r: Num, anti: bool) -> anyhow::Result<Num> {
        if anti {
            Ok(l.div(&r)?)
        } else {
            Ok(l.mul(&r)?)
        }
    }
}

enum Summable<T: SumOp> {
    Empty(PhantomData<T>),
    Bool(EventMeta, u64),
    Num(EventMeta, Num),
    Error(anyhow::Error),
}

impl<T: SumOp> AddAssign<&Value> for Summable<T> {
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: &Value) {
        let anti = rhs.is_anti();
        match std::mem::replace(self, Self::Empty(PhantomData)) {
            Summable::Empty(_) => {
                *self = rhs
                    .as_bool()
                    .map(|b| Self::Bool(rhs.meta().clone(), T::bool(0, b, anti)))
                    .or_else(|_| rhs.as_number().map(|n| Self::Num(rhs.meta().clone(), n)))
                    .unwrap_or_else(Self::Error)
            }
            Summable::Bool(mut meta, b) => {
                *self = rhs
                    .as_bool()
                    .map(|o| {
                        meta += rhs.meta();
                        Self::Bool(meta, T::bool(b, o, anti))
                    })
                    .unwrap_or_else(Self::Error)
            }
            Summable::Num(mut meta, n) => {
                *self = rhs
                    .as_number()
                    .and_then(|o| {
                        let result = T::num(n, o, anti)?;
                        meta += rhs.meta();
                        Ok(Self::Num(meta, result))
                    })
                    .unwrap_or_else(Self::Error)
            }
            Summable::Error(e) => *self = Self::Error(e),
        }
    }
}

impl<T: SumOp> Default for Summable<T> {
    fn default() -> Self {
        Summable::Empty(PhantomData)
    }
}

#[derive(Default)]
struct Sum<T: SumOp>(Summable<T>);
impl<T: SumOp> Aggregator for Sum<T> {
    fn feed(&mut self, input: Value) -> anyhow::Result<()> {
        self.0 += &input;
        Ok(())
    }

    fn flush(&mut self, cx: &Context) -> anyhow::Result<Value> {
        match &self.0 {
            Summable::Empty(_) => Err(RuntimeError::NoValueYet.into()),
            Summable::Bool(meta, n) => Ok(Value::new_meta(
                cx.mk_cbor(|b| b.encode_bool(T::to_bool(*n))),
                meta.clone(),
            )),
            Summable::Num(meta, n) => Ok(Value::new_meta(cx.number(n), meta.clone())),
            Summable::Error(e) => Err(anyhow!("incompatible types in sum: {}", e)),
        }
    }

    fn has_value(&self) -> bool {
        true
    }
}

struct First(Option<Value>);
impl Aggregator for First {
    fn feed(&mut self, input: Value) -> anyhow::Result<()> {
        if input.is_anti() {
            return Err(RuntimeFailure::AntiInputInFirst.into());
        }
        if let Some(v) = &mut self.0 {
            if input.min_key() < v.min_key() || input.min_key() == v.min_key() && input.max_key() < v.max_key() {
                *v = input;
            }
        } else {
            self.0 = Some(input);
        }
        Ok(())
    }

    fn flush(&mut self, _cx: &Context) -> anyhow::Result<Value> {
        self.0.clone().ok_or_else(|| RuntimeError::NoValueYet.into())
    }

    fn has_value(&self) -> bool {
        self.0.is_some()
    }
}

struct Last(Option<Value>);
impl Aggregator for Last {
    fn feed(&mut self, input: Value) -> anyhow::Result<()> {
        if input.is_anti() {
            return Err(RuntimeFailure::AntiInputInLast.into());
        }
        if let Some(v) = &mut self.0 {
            if input.max_key() > v.max_key() || input.max_key() == v.max_key() && input.min_key() > v.min_key() {
                *v = input;
            }
        } else {
            self.0 = Some(input);
        }
        Ok(())
    }

    fn flush(&mut self, _cx: &Context) -> anyhow::Result<Value> {
        self.0.clone().ok_or_else(|| RuntimeError::NoValueYet.into())
    }

    fn has_value(&self) -> bool {
        self.0.is_some()
    }
}

struct Min(Option<anyhow::Result<Value>>);
impl Aggregator for Min {
    fn feed(&mut self, input: Value) -> anyhow::Result<()> {
        if input.is_anti() {
            return Err(RuntimeFailure::AntiInputInMin.into());
        }
        self.0 = match self.0.take() {
            Some(Ok(v)) => match v.partial_cmp(&input) {
                Some(o) => Some(Ok(if o == Ordering::Greater { input } else { v })),
                None => Some(Err(anyhow!("cannot compare {} to {}", v, input))),
            },
            Some(Err(e)) => Some(Err(e)),
            None => Some(Ok(input)),
        };
        Ok(())
    }

    fn flush(&mut self, _cx: &Context) -> anyhow::Result<Value> {
        self.0
            .as_ref()
            .ok_or_else(|| RuntimeError::NoValueYet.into())
            .and_then(|r| match r {
                Ok(v) => Ok(v.clone()),
                Err(e) => Err(anyhow!("incompatible types in min: {}", e)),
            })
    }

    fn has_value(&self) -> bool {
        self.0.is_some()
    }
}

struct Max(Option<anyhow::Result<Value>>);
impl Aggregator for Max {
    fn feed(&mut self, input: Value) -> anyhow::Result<()> {
        if input.is_anti() {
            return Err(RuntimeFailure::AntiInputInMax.into());
        }
        self.0 = match self.0.take() {
            Some(Ok(v)) => match v.partial_cmp(&input) {
                Some(o) => Some(Ok(if o == Ordering::Less { input } else { v })),
                None => Some(Err(anyhow!("cannot compare {} to {}", v, input))),
            },
            Some(Err(e)) => Some(Err(e)),
            None => Some(Ok(input)),
        };
        Ok(())
    }

    fn flush(&mut self, _cx: &Context) -> anyhow::Result<Value> {
        self.0
            .as_ref()
            .ok_or_else(|| RuntimeError::NoValueYet.into())
            .and_then(|r| match r {
                Ok(v) => Ok(v.clone()),
                Err(e) => Err(anyhow!("incompatible types in max: {}", e)),
            })
    }

    fn has_value(&self) -> bool {
        self.0.is_some()
    }
}

struct AggrState {
    key: Arc<(AggrOp, SimpleExpr)>,
    aggregator: Box<dyn Aggregator + Send + Sync + 'static>,
    variable: u32,
}

struct Aggregate {
    expr: SimpleExpr,
    state: Vec<AggrState>,
    order: Option<Order>,
    previous: Option<Value>,
}
impl super::Processor for Aggregate {
    fn apply<'a, 'b: 'a>(&'a mut self, cx: &'a mut Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>> {
        async move {
            /*
             * Anti-flag propagation:
             *
             * If we get an anti-input, the specific aggregator will either ingest it or
             * emit an error. Aggregators always build on a (positive) set of inputs, so
             * they cannot contain an anti-value.
             */
            let anti = cx
                .lookup_opt("_")
                .map(|v| v.as_ref().map(|v| v.is_anti()).unwrap_or_default())
                .unwrap_or_default();
            let mut errors = vec![];
            for aggr in self.state.iter_mut() {
                match cx.eval(&aggr.key.1).await {
                    Ok(mut v) => {
                        if anti {
                            v.anti();
                        }
                        if let Err(e) = aggr.aggregator.feed(v) {
                            errors.push(Err(e))
                        }
                    }
                    Err(e) => errors.push(Err(e)),
                }
            }
            errors
        }
        .boxed()
    }

    fn flush<'a, 'b: 'a>(&'a mut self, cx: &'a mut Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>> {
        async move {
            let mut cx = cx.child();
            for aggr in self.state.iter_mut() {
                cx.bind_placeholder(format!("!{}", aggr.variable), aggr.aggregator.flush(&cx));
            }
            let v = match cx.eval(&self.expr).await {
                Ok(v) => v,
                e => return vec![e],
            };
            if let Some(mut p) = self.previous.take() {
                if p == v {
                    self.previous = Some(p);
                    vec![]
                } else {
                    self.previous = Some(v.clone());
                    p.anti();
                    vec![Ok(p), Ok(v)]
                }
            } else {
                self.previous = Some(v.clone());
                vec![Ok(v)]
            }
        }
        .boxed()
    }

    fn preferred_order(&self) -> Option<Order> {
        self.order
    }

    fn is_done(&self, order: Order) -> bool {
        Some(order) == self.order && self.state.iter().all(|a| a.aggregator.has_value()) || self.state.is_empty()
    }
}

pub(super) fn aggregate(expr: &SimpleExpr) -> Box<dyn super::Processor> {
    struct G<'a> {
        state: &'a mut Vec<AggrState>,
        counter: &'a mut u32,
    }
    impl<'a> Galactus for G<'a> {
        fn visit_expr(&mut self, expr: &SimpleExpr) -> Tactic<SimpleExpr, Self> {
            match expr {
                SimpleExpr::AggrOp(a) => {
                    let name = match self.state.binary_search_by_key(&a, |x| &x.key) {
                        Ok(found) => self.state[found].variable,
                        Err(idx) => {
                            let aggregator: Box<dyn Aggregator + Send + Sync> = match a.0 {
                                AggrOp::Sum => Box::new(Sum::<AddOp>::default()),
                                AggrOp::Prod => Box::new(Sum::<MulOp>::default()),
                                AggrOp::Min => Box::new(Min(None)),
                                AggrOp::Max => Box::new(Max(None)),
                                AggrOp::First => Box::new(First(None)),
                                AggrOp::Last => Box::new(Last(None)),
                            };
                            *self.counter += 1;
                            self.state.insert(
                                idx,
                                AggrState {
                                    key: a.clone(),
                                    aggregator,
                                    variable: *self.counter,
                                },
                            );
                            *self.counter
                        }
                    };
                    // it is important that these internal variables are not legal in user queries,
                    // hence the exclamation mark
                    Tactic::Devour(SimpleExpr::Variable(Var::internal(format!("!{}", name))))
                }
                // leave sub-queries alone
                //
                // This is about delineated contexts: an AGGREGATE expression treats the AggrOps
                // (like LAST()) specially, but the expression may also include sub-queries that
                // run once the aggregation is finished. Inside those sub-queries there might be
                // an AGGREGATE step, but it is not our place to touch that — it will be treated
                // properly when its evaluation time comes.
                SimpleExpr::SubQuery(_) => Tactic::KeepAsIs,
                _ => Tactic::Scrutinise,
            }
        }
    }

    let mut state = Vec::<AggrState>::new();
    let mut counter: u32 = 0;

    let expr = expr
        .rewrite(&mut G {
            state: &mut state,
            counter: &mut counter,
        })
        .0;

    let order = {
        let mut first = false;
        let mut last = false;
        let mut other = false;
        for s in &state {
            match s.key.0 {
                AggrOp::Sum => other = true,
                AggrOp::Prod => other = true,
                AggrOp::Min => other = true,
                AggrOp::Max => other = true,
                AggrOp::First => first = true,
                AggrOp::Last => last = true,
            }
        }
        if other || first && last {
            None
        } else if first {
            Some(Order::Asc)
        } else if last {
            Some(Order::Desc)
        } else {
            None
        }
    };

    Box::new(Aggregate {
        expr,
        state,
        order,
        previous: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        eval::RootContext,
        operation::{Operation, Processor},
        query::Query,
    };
    use actyx_sdk::{app_id, language, tags, EventKey, Metadata, NodeId};
    use swarm::event_store_ref::EventStoreRef;

    fn a(s: &str) -> Box<dyn Processor> {
        let s = format!("FROM 'x' AGGREGATE {}", s);
        let q = Query::from(language::Query::parse(&s).unwrap(), app_id!("com.actyx.test")).0;
        match q.stages.into_iter().next().unwrap() {
            Operation::Aggregate(a) => aggregate(&a),
            _ => panic!(),
        }
    }
    fn store() -> EventStoreRef {
        EventStoreRef::new(|_x| Err(swarm::event_store_ref::Error::Aborted))
    }
    fn ctx() -> RootContext {
        Context::new(store())
    }
    async fn apply<'a, 'b: 'a>(a: &'a mut dyn Processor, cx: &'a mut Context<'b>, v: u64, t: u64) -> Vec<Value> {
        cx.bind(
            "_",
            Value::new_meta(
                cx.mk_cbor(|b| b.encode_u64(v)),
                EventMeta::Event {
                    key: EventKey {
                        lamport: t.into(),
                        stream: NodeId::default().stream(0.into()),
                        offset: 0.into(),
                    },
                    meta: Metadata {
                        timestamp: 0.into(),
                        tags: tags!(),
                        app_id: app_id!("x"),
                    },
                },
            ),
        );
        a.apply(cx).await.into_iter().collect::<anyhow::Result<_>>().unwrap()
    }
    async fn flush<'a, 'b: 'a>(a: &'a mut dyn Processor, cx: &'a mut Context<'b>) -> String {
        let v = a
            .flush(cx)
            .await
            .into_iter()
            .map(|r| {
                let v = r.unwrap();
                if v.is_anti() {
                    format!("!{}", v.cbor())
                } else {
                    v.cbor().to_string()
                }
            })
            .collect::<Vec<_>>();
        v.join(",")
    }

    #[tokio::test]
    async fn sum() {
        let mut s = a("42 - SUM(_ * 2)");
        let cx = ctx();
        let mut cx = cx.child();

        assert_eq!(apply(&mut *s, &mut cx, 1, 1).await, vec![]);
        assert_eq!(apply(&mut *s, &mut cx, 2, 2).await, vec![]);
        assert_eq!(flush(&mut *s, &mut cx).await, "36");

        let mut s = a("CASE SUM(_ ≥ 2) => 11 CASE TRUE => 12 ENDCASE");

        assert_eq!(apply(&mut *s, &mut cx, 1, 3).await, vec![]);
        assert_eq!(flush(&mut *s, &mut cx).await, "12");
        assert_eq!(apply(&mut *s, &mut cx, 2, 4).await, vec![]);
        assert_eq!(flush(&mut *s, &mut cx).await, "!12,11");
    }

    #[tokio::test]
    async fn product() {
        let mut s = a("42 - PRODUCT(_ * 2)");
        let cx = ctx();
        let mut cx = cx.child();

        assert_eq!(apply(&mut *s, &mut cx, 1, 1).await, vec![]);
        assert_eq!(apply(&mut *s, &mut cx, 2, 2).await, vec![]);
        assert_eq!(flush(&mut *s, &mut cx).await, "34");

        let mut s = a("CASE PRODUCT(_ ≥ 2) => 11 CASE TRUE => 12 ENDCASE");

        assert_eq!(apply(&mut *s, &mut cx, 2, 3).await, vec![]);
        assert_eq!(flush(&mut *s, &mut cx).await, "11");
        assert_eq!(apply(&mut *s, &mut cx, 1, 4).await, vec![]);
        assert_eq!(flush(&mut *s, &mut cx).await, "!11,12");
    }

    #[tokio::test]
    async fn min_max() {
        let mut s = a("[FIRST(_), LAST(_), MIN(_), MAX(_)]");
        let cx = ctx();
        let mut cx = cx.child();

        assert_eq!(apply(&mut *s, &mut cx, 2, 1).await, vec![]);
        assert_eq!(flush(&mut *s, &mut cx).await, "[2, 2, 2, 2]");
        assert_eq!(apply(&mut *s, &mut cx, 1, 2).await, vec![]);
        assert_eq!(flush(&mut *s, &mut cx).await, "![2, 2, 2, 2],[2, 1, 1, 2]");
        assert_eq!(apply(&mut *s, &mut cx, 4, 3).await, vec![]);
        assert_eq!(flush(&mut *s, &mut cx).await, "![2, 1, 1, 2],[2, 4, 1, 4]");
        assert_eq!(apply(&mut *s, &mut cx, 3, 4).await, vec![]);
        assert_eq!(flush(&mut *s, &mut cx).await, "![2, 4, 1, 4],[2, 3, 1, 4]");
    }
}
