use crate::{eval::Context, value::Value};
use actyx_sdk::language::{AggrOp, Num, SimpleExpr, Traverse};
use anyhow::{anyhow, bail};
use cbor_data::{Encoder, Writer};
use std::{cmp::Ordering, collections::BTreeMap, marker::PhantomData, ops::AddAssign};

pub trait Aggregator {
    fn feed(&mut self, input: Value) -> anyhow::Result<()>;
    fn flush(&mut self, cx: &Context) -> anyhow::Result<Value>;
}

impl Aggregator for () {
    fn feed(&mut self, _input: Value) -> anyhow::Result<()> {
        Ok(())
    }

    fn flush(&mut self, cx: &Context) -> anyhow::Result<Value> {
        Ok(cx.value(|b| b.encode_u64(42)))
    }
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

#[derive(Default)]
struct Array(Vec<Value>);
impl Aggregator for Array {
    fn feed(&mut self, input: Value) -> anyhow::Result<()> {
        match self.0.first().map(|f| f.kind()) {
            Some(kind) if kind != input.kind() => anyhow::bail!("Expected \"{}\", found \"{}\"", kind, input.kind()),
            _ => self.0.push(input),
        }
        Ok(())
    }

    fn flush(&mut self, cx: &Context) -> anyhow::Result<Value> {
        Ok(cx.value(|b| {
            b.encode_array(|b| {
                for item in self.0.drain(..) {
                    b.write_trusting(item.as_slice());
                }
            })
        }))
    }
}

pub type AggrState = BTreeMap<(AggrOp, SimpleExpr), Box<dyn Aggregator + Send>>;

pub struct Aggregate {
    pub expr: SimpleExpr,
    state: AggrState,
}

impl PartialEq for Aggregate {
    fn eq(&self, other: &Self) -> bool {
        self.expr == other.expr
    }
}

impl std::fmt::Debug for Aggregate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Aggregate({:?})", self.expr)
    }
}

impl Aggregate {
    pub fn new(expr: SimpleExpr) -> Self {
        let mut state: AggrState = BTreeMap::new();
        expr.traverse(&mut |e| match e {
            SimpleExpr::AggrOp(a) => {
                let op: Box<dyn Aggregator + Send> = match a.0 {
                    AggrOp::Sum => Box::new(Sum::<AddOp>::default()),
                    AggrOp::Prod => Box::new(Sum::<MulOp>::default()),
                    AggrOp::Min => Box::new(Min(None)),
                    AggrOp::Max => Box::new(Max(None)),
                    AggrOp::First => Box::new(First(None)),
                    AggrOp::Last => Box::new(Last(None)),
                    AggrOp::Array => Box::new(Array::default()),
                };
                state.insert((a.0, a.1.clone()), op);
                Traverse::Stop
            }
            _ => Traverse::Descend,
        });
        Self { expr, state }
    }

    pub fn apply(&mut self, cx: &Context, input: Value) -> Vec<anyhow::Result<Value>> {
        let mut cx = cx.child();
        cx.bind("_", input);

        let mut errors = vec![];
        fn log_error(errors: &mut Vec<anyhow::Result<Value>>, f: impl FnOnce() -> anyhow::Result<()>) {
            match f() {
                Ok(_) => {}
                Err(e) => errors.push(Err(e)),
            }
        }

        for ((_op, expr), agg) in &mut self.state {
            log_error(&mut errors, || {
                let v = cx.eval(expr)?;
                agg.feed(v)?;
                Ok(())
            });
        }
        errors
    }

    pub fn flush(&mut self, cx: &Context) -> anyhow::Result<Value> {
        let mut cx = cx.child();
        cx.bind_aggregation(&mut self.state);
        cx.eval(&self.expr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{operation::Operation, query::Query};
    use actyx_sdk::{
        language::{self, SortKey},
        NodeId,
    };

    fn a(s: &str) -> Aggregate {
        let s = format!("FROM 'x' AGGREGATE {}", s);
        let q = Query::from(s.parse::<language::Query>().unwrap());
        match q.stages.into_iter().next().unwrap() {
            Operation::Aggregate(a) => a,
            _ => panic!(),
        }
    }
    fn ctx() -> Context<'static> {
        Context::new(SortKey {
            lamport: Default::default(),
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(0.into()),
        })
    }
    fn apply(a: &mut Aggregate, cx: &mut Context, v: u64) -> Vec<Value> {
        cx.incr();
        a.apply(cx, cx.value(|b| b.encode_u64(v)))
            .into_iter()
            .collect::<anyhow::Result<_>>()
            .unwrap()
    }

    fn apply_str(a: &mut Aggregate, cx: &mut Context, v: &str) -> anyhow::Result<Vec<Value>> {
        cx.incr();
        a.apply(cx, cx.value(|b| b.encode_str(v)))
            .into_iter()
            .collect::<anyhow::Result<_>>()
    }
    fn flush(a: &mut Aggregate, cx: &Context) -> String {
        a.flush(cx).unwrap().cbor().to_string()
    }

    #[test]
    fn sum() {
        let mut s = a("42 - SUM(_ * 2)");
        let mut cx = ctx();

        assert_eq!(apply(&mut s, &mut cx, 1), vec![]);
        assert_eq!(apply(&mut s, &mut cx, 2), vec![]);
        assert_eq!(flush(&mut s, &cx), "36");

        let mut s = a("CASE SUM(_ ≥ 2) => 11 CASE TRUE => 12 ENDCASE");

        assert_eq!(apply(&mut s, &mut cx, 1), vec![]);
        assert_eq!(flush(&mut s, &cx), "12");
        assert_eq!(apply(&mut s, &mut cx, 2), vec![]);
        assert_eq!(flush(&mut s, &cx), "11");
    }

    #[test]
    fn product() {
        let mut s = a("42 - PRODUCT(_ * 2)");
        let mut cx = ctx();

        assert_eq!(apply(&mut s, &mut cx, 1), vec![]);
        assert_eq!(apply(&mut s, &mut cx, 2), vec![]);
        assert_eq!(flush(&mut s, &cx), "34");

        let mut s = a("CASE PRODUCT(_ ≥ 2) => 11 CASE TRUE => 12 ENDCASE");

        assert_eq!(apply(&mut s, &mut cx, 2), vec![]);
        assert_eq!(flush(&mut s, &cx), "11");
        assert_eq!(apply(&mut s, &mut cx, 1), vec![]);
        assert_eq!(flush(&mut s, &cx), "12");
    }

    #[test]
    fn min_max() {
        let mut s = a("[FIRST(_), LAST(_), MIN(_), MAX(_)]");
        let mut cx = ctx();

        assert_eq!(apply(&mut s, &mut cx, 2), vec![]);
        assert_eq!(flush(&mut s, &cx), "[2, 2, 2, 2]");
        assert_eq!(apply(&mut s, &mut cx, 1), vec![]);
        assert_eq!(flush(&mut s, &cx), "[2, 1, 1, 2]");
        assert_eq!(apply(&mut s, &mut cx, 4), vec![]);
        assert_eq!(flush(&mut s, &cx), "[2, 4, 1, 4]");
        assert_eq!(apply(&mut s, &mut cx, 3), vec![]);
        assert_eq!(flush(&mut s, &cx), "[2, 3, 1, 4]");
    }

    #[test]
    fn array() {
        let mut s = a("ARRAY(_)");
        let mut cx = ctx();

        for i in 1..=4 {
            assert_eq!(apply(&mut s, &mut cx, i), vec![]);
        }
        assert_eq!(flush(&mut s, &cx), "[1, 2, 3, 4]");

        assert_eq!(apply(&mut s, &mut cx, 1), vec![]);
        assert!(apply_str(&mut s, &mut cx, "str").is_err());
    }
}
