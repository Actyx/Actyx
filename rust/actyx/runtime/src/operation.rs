use crate::{eval::Context, value::Value};
use actyx_sdk::language::{NonEmptyVec, SimpleExpr};

mod aggregate;
pub use aggregate::{AggrState, Aggregate, Aggregator};

#[derive(Debug, PartialEq)]
pub enum Operation {
    Filter(Filter),
    Select(Select),
    Aggregate(aggregate::Aggregate),
}

impl Operation {
    pub async fn apply(&mut self, cx: &mut Context<'_>) -> Vec<anyhow::Result<Value>> {
        match self {
            Operation::Filter(f) => f.apply(cx).await.into_iter().collect(),
            Operation::Select(s) => s.apply(cx).await,
            Operation::Aggregate(a) => a.apply(cx).await,
        }
    }

    pub async fn flush(&mut self, cx: &Context<'_>) -> Vec<anyhow::Result<Value>> {
        match self {
            Operation::Aggregate(a) => vec![a.flush(cx).await],
            _ => vec![],
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

    pub async fn apply(&self, cx: &mut Context<'_>) -> Option<anyhow::Result<Value>> {
        cx.eval(&self.expr)
            .await
            .and_then(move |v| {
                if v.as_bool()? {
                    Ok(cx.lookup("_").cloned())
                } else {
                    Ok(None)
                }
            })
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

    pub async fn apply(&self, cx: &mut Context<'_>) -> Vec<anyhow::Result<Value>> {
        let mut v = vec![];
        for expr in self.exprs.iter() {
            v.push(cx.eval(expr).await)
        }
        v
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

    #[tokio::test]
    async fn filter() {
        let f = Filter::new(simple_expr("_ > 5 + a"));
        let mut cx = Context::new(key());
        cx.bind("a", cx.value(|b| b.encode_f64(3.0)));

        cx.bind("_", cx.value(|b| b.encode_i64(8)));
        assert!(f.apply(&mut cx).await.is_none());

        let v = cx.value(|b| b.encode_i64(9));
        cx.bind("_", v.clone());
        assert_eq!(f.apply(&mut cx).await.unwrap().unwrap(), v);
    }

    #[tokio::test]
    async fn select() {
        let s = Select::new(vec![simple_expr("_.x + a")].try_into().unwrap());
        let mut cx = Context::new(key());
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
            s.apply(&mut cx).await.into_iter().next().unwrap().unwrap(),
            cx.value(|b| b.encode_f64(2.5))
        );
    }
}
