use crate::{eval::Context, value::Value};
use actyx_sdk::language::SimpleExpr;
use anyhow::Result;

pub enum Operation {
    Filter(Filter),
    Select(Select),
}

impl Operation {
    pub fn apply<'a>(&'a self, cx: &'a Context, input: Value) -> (Vec<Value>, &'a Context) {
        match self {
            Operation::Filter(f) => (f.apply(cx, input).unwrap_or(None).into_iter().collect(), cx),
            Operation::Select(s) => (s.apply(cx, input).map(|x| vec![x]).unwrap_or_default(), cx),
        }
    }
}

pub struct Filter {
    expr: SimpleExpr,
}

impl Filter {
    pub fn new(expr: SimpleExpr) -> Self {
        Self { expr }
    }

    pub fn apply(&self, cx: &Context, input: Value) -> Result<Option<Value>> {
        let mut cx = cx.child();
        cx.bind("_", input.clone());
        cx.eval(&self.expr)
            .and_then(move |v| if v.as_bool()? { Ok(Some(input)) } else { Ok(None) })
    }
}

pub struct Select {
    expr: SimpleExpr,
}

impl Select {
    pub fn new(expr: SimpleExpr) -> Self {
        Self { expr }
    }

    pub fn apply(&self, cx: &Context, input: Value) -> Result<Value> {
        let mut cx = cx.child();
        cx.bind("_", input);
        cx.eval(&self.expr)
    }
}

#[cfg(test)]
mod tests {
    use actyx_sdk::{EventKey, NodeId};
    use cbor_data::Encoder;

    use super::*;

    fn simple_expr(s: &str) -> SimpleExpr {
        s.parse::<SimpleExpr>().unwrap()
    }

    fn key() -> EventKey {
        EventKey {
            lamport: Default::default(),
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(0.into()),
            offset: Default::default(),
        }
    }

    #[test]
    fn filter() {
        let f = Filter::new(simple_expr("_ > 5 + a"));
        let mut cx = Context::new(key());
        cx.bind("a", cx.value(|b| b.encode_f64(3.0)));

        assert_eq!(f.apply(&cx, cx.value(|b| b.encode_i64(8))).unwrap(), None);

        let v = cx.value(|b| b.encode_i64(9));
        assert_eq!(f.apply(&cx, v.clone()).unwrap(), Some(v));
    }

    #[test]
    fn select() {
        let s = Select::new(simple_expr("_.x + a"));
        let mut cx = Context::new(key());
        cx.bind("a", cx.value(|b| b.encode_f64(0.5)));

        assert_eq!(
            s.apply(
                &cx,
                cx.value(|b| b.encode_dict(|b| {
                    b.with_key("x", |b| b.encode_u64(2));
                }))
            )
            .unwrap(),
            cx.value(|b| b.encode_f64(2.5))
        );
    }
}
