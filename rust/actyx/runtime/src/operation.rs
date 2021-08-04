use crate::{eval::Context, value::Value};
use actyx_sdk::language::{AggrOp, NonEmptyVec, SimpleExpr, Traverse};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq)]
pub enum Operation {
    Filter(Filter),
    Select(Select),
    Aggregate(Aggregate),
}

impl Operation {
    pub fn apply(&mut self, cx: &Context, input: Value) -> Vec<anyhow::Result<Value>> {
        match self {
            Operation::Filter(f) => f.apply(cx, input).into_iter().collect(),
            Operation::Select(s) => s.apply(cx, input),
            Operation::Aggregate(a) => a.apply(cx, input),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Filter {
    pub expr: SimpleExpr,
}

impl Filter {
    pub fn new(expr: SimpleExpr) -> Self {
        Self { expr }
    }

    pub fn apply(&self, cx: &Context, input: Value) -> Option<anyhow::Result<Value>> {
        let mut cx = cx.child();
        cx.bind("_", input.clone());
        cx.eval(&self.expr)
            .and_then(move |v| if v.as_bool()? { Ok(Some(input)) } else { Ok(None) })
            .transpose()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Select {
    pub exprs: NonEmptyVec<SimpleExpr>,
}

impl Select {
    pub fn new(exprs: NonEmptyVec<SimpleExpr>) -> Self {
        Self { exprs }
    }

    pub fn apply(&self, cx: &Context, input: Value) -> Vec<anyhow::Result<Value>> {
        let mut cx = cx.child();
        cx.bind("_", input);
        self.exprs.iter().map(|expr| cx.eval(expr)).collect()
    }
}

pub trait Aggregator {
    fn feed(&mut self, input: Value) -> anyhow::Result<()>;
}
impl Aggregator for () {
    fn feed(&mut self, _input: Value) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct Aggregate {
    pub expr: SimpleExpr,
    state: BTreeMap<(AggrOp, SimpleExpr), Box<dyn Aggregator + Send>>,
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
        let mut state = BTreeMap::new();
        expr.traverse(&mut |e| match e {
            SimpleExpr::AggrOp(a) => {
                state.insert((a.0, a.1.clone()), Box::new(()) as Box<dyn Aggregator + Send>);
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
}

#[cfg(test)]
mod tests {
    use actyx_sdk::{language::SortKey, NodeId};
    use cbor_data::Encoder;

    use super::*;
    use std::convert::TryInto;

    fn simple_expr(s: &str) -> SimpleExpr {
        s.parse::<SimpleExpr>().unwrap()
    }

    fn key() -> SortKey {
        SortKey {
            lamport: Default::default(),
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(0.into()),
        }
    }

    #[test]
    fn filter() {
        let f = Filter::new(simple_expr("_ > 5 + a"));
        let mut cx = Context::new(key());
        cx.bind("a", cx.value(|b| b.encode_f64(3.0)));

        assert!(f.apply(&cx, cx.value(|b| b.encode_i64(8))).is_none());

        let v = cx.value(|b| b.encode_i64(9));
        assert_eq!(f.apply(&cx, v.clone()).unwrap().unwrap(), v);
    }

    #[test]
    fn select() {
        let s = Select::new(vec![simple_expr("_.x + a")].try_into().unwrap());
        let mut cx = Context::new(key());
        cx.bind("a", cx.value(|b| b.encode_f64(0.5)));

        assert_eq!(
            s.apply(
                &cx,
                cx.value(|b| b.encode_dict(|b| {
                    b.with_key("x", |b| b.encode_u64(2));
                }))
            )
            .into_iter()
            .next()
            .unwrap()
            .unwrap(),
            cx.value(|b| b.encode_f64(2.5))
        );
    }
}
