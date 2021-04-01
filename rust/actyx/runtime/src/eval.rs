use crate::value::Value;
use actyxos_sdk::{
    language::{Number, SimpleExpr},
    EventKey,
};
use anyhow::{anyhow, bail, Result};
use cbor_data::{CborBuilder, CborOwned, Encoder, WithOutput, Writer};
use std::collections::BTreeMap;

pub struct Context<'a> {
    sort_key: EventKey,
    bindings: BTreeMap<String, Value>,
    parent: Option<&'a Context<'a>>,
}

impl<'a> Context<'a> {
    pub fn new(sort_key: EventKey) -> Self {
        Self {
            sort_key,
            bindings: BTreeMap::new(),
            parent: None,
        }
    }

    pub fn child(&'a self) -> Self {
        Self {
            sort_key: self.sort_key,
            bindings: BTreeMap::new(),
            parent: Some(self),
        }
    }

    pub fn value(&self, f: impl FnOnce(CborBuilder<WithOutput>) -> CborOwned) -> Value {
        Value::new(self.sort_key, f)
    }

    pub fn bind(&mut self, name: impl Into<String>, value: Value) {
        self.bindings.insert(name.into(), value);
    }

    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.bindings
            .get(name)
            .or_else(|| self.parent.and_then(|c| c.lookup(name)))
    }

    pub fn eval(&self, expr: &SimpleExpr) -> Result<Value> {
        match expr {
            SimpleExpr::Path(p) => {
                let v = self
                    .lookup(&*p.head)
                    .ok_or_else(|| anyhow!("variable '{}' is not bound", p.head))?;
                let idx = v
                    .index(&p.tail)
                    .ok_or_else(|| anyhow!("path {:?} does not exist in value {}", p.tail, v.value()))?;
                Ok(self.value(|b| b.write_trusting(idx.bytes)))
            }
            SimpleExpr::Number(n) => match n {
                Number::Decimal(d) => Ok(self.value(|b| b.encode_f64(*d))),
                Number::Natural(n) => Ok(self.value(|b| b.encode_u64(*n))),
            },
            SimpleExpr::String(_) => bail!("string not yet implemented"),
            SimpleExpr::Object(_) => bail!("object construction not yet implemented"),
            SimpleExpr::Array(_) => bail!("array construction not yet implemented"),
            SimpleExpr::Add(a) => self.eval(&a.0)?.add(&self.eval(&a.1)?),
            SimpleExpr::Sub(_) => bail!("subtraction not yet implemented"),
            SimpleExpr::Mul(_) => bail!("multiplication not yet implemented"),
            SimpleExpr::Div(_) => bail!("division not yet implemented"),
            SimpleExpr::Mod(_) => bail!("modulus not yet implemented"),
            SimpleExpr::Pow(_) => bail!("exponentiation not yet implemented"),
            SimpleExpr::And(_) => bail!("logical and not yet implemented"),
            SimpleExpr::Or(_) => bail!("logical or not yet implemented"),
            SimpleExpr::Not(_) => bail!("negation not yet implemented"),
            SimpleExpr::Xor(_) => bail!("exclusive or not yet implemented"),
            SimpleExpr::Lt(a) => self.eval(&a.0)?.lt(&self.eval(&a.1)?),
            SimpleExpr::Le(_) => bail!("less or equal not yet implemented"),
            SimpleExpr::Gt(a) => self.eval(&a.0)?.gt(&self.eval(&a.1)?),
            SimpleExpr::Ge(_) => bail!("greater or equal not yet implemented"),
            SimpleExpr::Eq(_) => bail!("equality not yet implemented"),
            SimpleExpr::Ne(_) => bail!("inequality not yet implemented"),
        }
    }
}

#[cfg(test)]
mod tests {
    use actyxos_sdk::language::{expression, Expression};

    use super::*;

    fn expr(s: &str) -> SimpleExpr {
        match expression(s).unwrap() {
            Expression::Simple(s) => s,
            Expression::Query(_) => panic!("expected simple expression"),
        }
    }

    fn eval(cx: &Context, s: &str) -> Result<String> {
        cx.eval(&expr(s)).map(|x| x.value().to_string())
    }

    #[test]
    fn simple() {
        let mut cx = Context::new(EventKey::default());
        cx.bind(
            "x",
            Value::new(cx.sort_key, |b| {
                b.encode_dict(|b| {
                    b.with_key("y", |b| b.encode_u64(42));
                })
            }),
        );

        assert_eq!(eval(&cx, "5+2.1+x.y").unwrap(), "49.1");

        assert_eq!(eval(&cx, "x").unwrap(), "{\"y\": 42}");

        let err = eval(&cx, "5+x").unwrap_err().to_string();
        assert!(err.contains("{\"y\": 42} is not a number"), "didn’t match: {}", err);

        let err = eval(&cx, "y").unwrap_err().to_string();
        assert!(err.contains("variable 'y' is not bound"), "didn’t match: {}", err);

        let err = eval(&cx, "x.a").unwrap_err().to_string();
        assert!(
            err.contains("path [Ident(\"a\")] does not exist in value {\"y\": 42}"),
            "didn’t match: {}",
            err
        );
    }
}
