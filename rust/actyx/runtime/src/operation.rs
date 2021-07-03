use crate::{eval::Context, value::Value};
use actyx_sdk::language::SimpleExpr;

pub enum Operation {
    Filter(Filter),
    Select(Select),
}

impl Operation {
    pub fn apply<'a>(&'a self, cx: &'a Context, input: Value) -> (Vec<anyhow::Result<Value>>, &'a Context) {
        match self {
            Operation::Filter(f) => (f.apply(cx, input).into_iter().collect(), cx),
            Operation::Select(s) => (s.apply(cx, input), cx),
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

    pub fn apply(&self, cx: &Context, input: Value) -> Option<anyhow::Result<Value>> {
        let mut cx = cx.child();
        cx.bind("_", input.clone());
        cx.eval(&self.expr)
            .and_then(move |v| if v.as_bool()? { Ok(Some(input)) } else { Ok(None) })
            .transpose()
    }
}

pub struct Select {
    exprs: Vec<SimpleExpr>,
}

impl Select {
    pub fn new(exprs: Vec<SimpleExpr>) -> Self {
        Self { exprs }
    }

    pub fn apply(&self, cx: &Context, input: Value) -> Vec<anyhow::Result<Value>> {
        let mut cx = cx.child();
        cx.bind("_", input);
        self.exprs.iter().map(|expr| cx.eval(expr)).collect()
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

        assert!(f.apply(&cx, cx.value(|b| b.encode_i64(8))).is_none());

        let v = cx.value(|b| b.encode_i64(9));
        assert_eq!(f.apply(&cx, v.clone()).unwrap().unwrap(), v);
    }

    #[test]
    fn select() {
        let s = Select::new(vec![simple_expr("_.x + a")]);
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
