use crate::{eval::Context, value::Value};
use actyx_sdk::language::{AggrOp, Num, SimpleExpr, Var};
use anyhow::{anyhow, bail};
use cbor_data::Encoder;
use futures::{future::BoxFuture, FutureExt};
use std::{cmp::Ordering, marker::PhantomData, ops::AddAssign, sync::Arc};

pub trait Aggregator {
    fn feed(&mut self, input: Value) -> anyhow::Result<()>;
    fn flush(&mut self, cx: &Context) -> anyhow::Result<Value>;
}

trait SumOp {
    fn bool(l: bool, r: bool) -> bool;
    fn num(l: Num, r: Num) -> anyhow::Result<Num>;
}
#[derive(Default)]
struct AddOp;
impl SumOp for AddOp {
    fn bool(l: bool, r: bool) -> bool {
        l || r
    }
    fn num(l: Num, r: Num) -> anyhow::Result<Num> {
        Ok(l.add(&r)?)
    }
}
#[derive(Default)]
struct MulOp;
impl SumOp for MulOp {
    fn bool(l: bool, r: bool) -> bool {
        l && r
    }
    fn num(l: Num, r: Num) -> anyhow::Result<Num> {
        Ok(l.mul(&r)?)
    }
}

enum Summable<T: SumOp> {
    Empty(PhantomData<T>),
    Bool(bool),
    Num(Num),
    Error(anyhow::Error),
}

impl<T: SumOp> AddAssign<&Value> for Summable<T> {
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, rhs: &Value) {
        match std::mem::replace(self, Self::Empty(PhantomData)) {
            Summable::Empty(_) => {
                *self = rhs
                    .as_bool()
                    .map(Self::Bool)
                    .or_else(|_| rhs.as_number().map(Self::Num))
                    .unwrap_or_else(Self::Error)
            }
            Summable::Bool(b) => {
                *self = rhs
                    .as_bool()
                    .map(|o| Self::Bool(T::bool(b, o)))
                    .unwrap_or_else(Self::Error)
            }
            Summable::Num(n) => {
                *self = rhs
                    .as_number()
                    .and_then(|o| Ok(Self::Num(T::num(n, o)?)))
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
            Summable::Empty(_) => bail!("no value added"),
            Summable::Bool(n) => Ok(cx.value(|b| b.encode_bool(*n))),
            Summable::Num(n) => Ok(cx.number(n)),
            Summable::Error(e) => Err(anyhow!("incompatible types in sum: {}", e)),
        }
    }
}

struct First(Option<Value>);
impl Aggregator for First {
    fn feed(&mut self, input: Value) -> anyhow::Result<()> {
        if let Some(v) = &mut self.0 {
            if input.key() < v.key() {
                *v = input;
            }
        } else {
            self.0 = Some(input);
        }
        Ok(())
    }

    fn flush(&mut self, _cx: &Context) -> anyhow::Result<Value> {
        self.0.clone().ok_or_else(|| anyhow!("no value added"))
    }
}

struct Last(Option<Value>);
impl Aggregator for Last {
    fn feed(&mut self, input: Value) -> anyhow::Result<()> {
        if let Some(v) = &mut self.0 {
            if input.key() > v.key() {
                *v = input;
            }
        } else {
            self.0 = Some(input);
        }
        Ok(())
    }

    fn flush(&mut self, _cx: &Context) -> anyhow::Result<Value> {
        self.0.clone().ok_or_else(|| anyhow!("no value added"))
    }
}

struct Min(Option<anyhow::Result<Value>>);
impl Aggregator for Min {
    fn feed(&mut self, input: Value) -> anyhow::Result<()> {
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
            .ok_or_else(|| anyhow!("no value added"))
            .and_then(|r| match r {
                Ok(v) => Ok(v.clone()),
                Err(e) => Err(anyhow!("incompatible types in min: {}", e)),
            })
    }
}

struct Max(Option<anyhow::Result<Value>>);
impl Aggregator for Max {
    fn feed(&mut self, input: Value) -> anyhow::Result<()> {
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
            .ok_or_else(|| anyhow!("no value added"))
            .and_then(|r| match r {
                Ok(v) => Ok(v.clone()),
                Err(e) => Err(anyhow!("incompatible types in max: {}", e)),
            })
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
}
impl super::Processor for Aggregate {
    fn apply<'a, 'b: 'a>(&'a mut self, cx: &'a Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>> {
        async move {
            let mut errors = vec![];
            for aggr in self.state.iter_mut() {
                match cx.eval(&aggr.key.1).await {
                    Ok(v) => {
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

    fn flush<'a, 'b: 'a>(&'a mut self, cx: &'a Context<'b>) -> BoxFuture<'a, Vec<anyhow::Result<Value>>> {
        async move {
            let mut cx = cx.child();
            for aggr in self.state.iter_mut() {
                cx.bind_placeholder(format!("!{}", aggr.variable), aggr.aggregator.flush(&cx));
            }
            vec![cx.eval(&self.expr).await]
        }
        .boxed()
    }
}

pub(super) fn aggregate(expr: &SimpleExpr) -> Box<dyn super::Processor> {
    let mut state = Vec::<AggrState>::new();
    let mut counter: u32 = 0;
    let expr = expr.rewrite(&mut |e| match e {
        SimpleExpr::AggrOp(a) => {
            let name = match state.binary_search_by_key(&a, |x| &x.key) {
                Ok(found) => state[found].variable,
                Err(idx) => {
                    let aggregator: Box<dyn Aggregator + Send + Sync> = match a.0 {
                        AggrOp::Sum => Box::new(Sum::<AddOp>::default()),
                        AggrOp::Prod => Box::new(Sum::<MulOp>::default()),
                        AggrOp::Min => Box::new(Min(None)),
                        AggrOp::Max => Box::new(Max(None)),
                        AggrOp::First => Box::new(First(None)),
                        AggrOp::Last => Box::new(Last(None)),
                    };
                    counter += 1;
                    state.insert(
                        idx,
                        AggrState {
                            key: a.clone(),
                            aggregator,
                            variable: counter,
                        },
                    );
                    counter
                }
            };
            // it is important that these internal variables are not legal in user queries,
            // hence the exclamation mark
            Some(SimpleExpr::Variable(Var::internal(format!("!{}", name))))
        }
        // leave sub-queries alone
        SimpleExpr::SubQuery(_) => Some(e.clone()),
        _ => None,
    });

    Box::new(Aggregate { expr, state })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        operation::{Operation, Processor},
        query::Query,
    };
    use actyx_sdk::{
        language::{self, SortKey},
        NodeId, OffsetMap,
    };
    use swarm::event_store_ref::EventStoreRef;

    fn a(s: &str) -> Box<dyn Processor> {
        let s = format!("FROM 'x' AGGREGATE {}", s);
        let q = Query::from(s.parse::<language::Query>().unwrap());
        match q.stages.into_iter().next().unwrap() {
            Operation::Aggregate(a) => aggregate(&a),
            _ => panic!(),
        }
    }
    fn store() -> EventStoreRef {
        EventStoreRef::new(|_x| Err(swarm::event_store_ref::Error::Aborted))
    }
    fn ctx() -> Context<'static> {
        Context::owned(
            SortKey {
                lamport: Default::default(),
                stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(0.into()),
            },
            store(),
            OffsetMap::empty(),
            OffsetMap::empty(),
        )
    }
    async fn apply<'a, 'b: 'a>(a: &'a mut dyn Processor, cx: &'a mut Context<'b>, v: u64) -> Vec<Value> {
        cx.incr();
        cx.bind("_", cx.value(|b| b.encode_u64(v)));
        a.apply(cx).await.into_iter().collect::<anyhow::Result<_>>().unwrap()
    }
    async fn flush<'a, 'b: 'a>(a: &'a mut dyn Processor, cx: &'a Context<'b>) -> String {
        a.flush(cx)
            .await
            .into_iter()
            .next()
            .unwrap()
            .unwrap()
            .cbor()
            .to_string()
    }

    #[tokio::test]
    async fn sum() {
        let mut s = a("42 - SUM(_ * 2)");
        let mut cx = ctx();

        assert_eq!(apply(&mut *s, &mut cx, 1).await, vec![]);
        assert_eq!(apply(&mut *s, &mut cx, 2).await, vec![]);
        assert_eq!(flush(&mut *s, &cx).await, "36");

        let mut s = a("CASE SUM(_ ≥ 2) => 11 CASE TRUE => 12 ENDCASE");

        assert_eq!(apply(&mut *s, &mut cx, 1).await, vec![]);
        assert_eq!(flush(&mut *s, &cx).await, "12");
        assert_eq!(apply(&mut *s, &mut cx, 2).await, vec![]);
        assert_eq!(flush(&mut *s, &cx).await, "11");
    }

    #[tokio::test]
    async fn product() {
        let mut s = a("42 - PRODUCT(_ * 2)");
        let mut cx = ctx();

        assert_eq!(apply(&mut *s, &mut cx, 1).await, vec![]);
        assert_eq!(apply(&mut *s, &mut cx, 2).await, vec![]);
        assert_eq!(flush(&mut *s, &cx).await, "34");

        let mut s = a("CASE PRODUCT(_ ≥ 2) => 11 CASE TRUE => 12 ENDCASE");

        assert_eq!(apply(&mut *s, &mut cx, 2).await, vec![]);
        assert_eq!(flush(&mut *s, &cx).await, "11");
        assert_eq!(apply(&mut *s, &mut cx, 1).await, vec![]);
        assert_eq!(flush(&mut *s, &cx).await, "12");
    }

    #[tokio::test]
    async fn min_max() {
        let mut s = a("[FIRST(_), LAST(_), MIN(_), MAX(_)]");
        let mut cx = ctx();

        assert_eq!(apply(&mut *s, &mut cx, 2).await, vec![]);
        assert_eq!(flush(&mut *s, &cx).await, "[2, 2, 2, 2]");
        assert_eq!(apply(&mut *s, &mut cx, 1).await, vec![]);
        assert_eq!(flush(&mut *s, &cx).await, "[2, 1, 1, 2]");
        assert_eq!(apply(&mut *s, &mut cx, 4).await, vec![]);
        assert_eq!(flush(&mut *s, &cx).await, "[2, 4, 1, 4]");
        assert_eq!(apply(&mut *s, &mut cx, 3).await, vec![]);
        assert_eq!(flush(&mut *s, &cx).await, "[2, 3, 1, 4]");
    }
}
