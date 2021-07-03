use crate::value::{Value, ValueKind};
use actyx_sdk::{
    language::{Indexing, Number, SimpleExpr},
    EventKey,
};
use anyhow::{anyhow, bail};
use cbor_data::{CborBuilder, CborOwned, Encoder, WithOutput, Writer};
use std::{cmp::Ordering, collections::BTreeMap};

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

    pub fn eval(&self, expr: &SimpleExpr) -> anyhow::Result<Value> {
        match expr {
            SimpleExpr::Var(v) => {
                let v = self.lookup(v).ok_or_else(|| anyhow!("variable '{}' is not bound", v))?;
                Ok(self.value(|b| b.write_trusting(v.as_slice())))
            }
            SimpleExpr::Indexing(Indexing { head, tail }) => {
                let mut v = self.eval(head)?;
                for i in tail {
                    v = match i {
                        actyx_sdk::language::Index::Ident(s) => v.index(s)?,
                        actyx_sdk::language::Index::Number(n) => v.index(&format!("{}", n))?,
                        actyx_sdk::language::Index::Expr(e) => {
                            let idx = self.eval(e)?;
                            match idx.kind() {
                                ValueKind::Number => v.index(&format!("{}", idx.value()))?,
                                ValueKind::String => v.index(idx.as_str()?)?,
                                _ => bail!("cannot index by {}", idx.value()),
                            }
                        }
                    };
                }
                Ok(self.value(|b| b.write_trusting(v.as_slice())))
            }
            SimpleExpr::Number(n) => match n {
                Number::Decimal(d) => Ok(self.value(|b| b.encode_f64(*d))),
                Number::Natural(n) => Ok(self.value(|b| b.encode_u64(*n))),
            },
            SimpleExpr::String(s) => Ok(self.value(|b| b.encode_str(s))),
            SimpleExpr::Object(a) => {
                let v = a
                    .props
                    .iter()
                    // TODO don’t evaluate overwritten properties
                    .map(|(n, e)| Ok((n.as_str(), self.eval(e)?)))
                    .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
                Ok(self.value(|b| {
                    b.encode_dict(|b| {
                        for (name, item) in v.iter() {
                            b.with_key(name, |b| b.write_trusting(item.as_slice()));
                        }
                    })
                }))
            }
            SimpleExpr::Array(a) => {
                let v = a
                    .items
                    .iter()
                    .map(|e| self.eval(e))
                    .collect::<anyhow::Result<Vec<_>>>()?;
                Ok(self.value(|b| {
                    b.encode_array(|b| {
                        for item in v.iter() {
                            b.write_trusting(item.as_slice());
                        }
                    })
                }))
            }
            SimpleExpr::Null => Ok(self.value(|b| b.write_null(None))),
            SimpleExpr::Bool(f) => Ok(self.value(|b| b.write_bool(*f, None))),
            SimpleExpr::Cases(v) => {
                for (pred, expr) in v.iter() {
                    let pred = self.eval(pred)?.as_bool()?;
                    if pred {
                        return self.eval(expr);
                    }
                }
                Err(anyhow!("no case matched"))
            }
            SimpleExpr::Add(a) => self.eval(&a.0)?.add(&self.eval(&a.1)?),
            SimpleExpr::Sub(a) => self.eval(&a.0)?.sub(&self.eval(&a.1)?),
            SimpleExpr::Mul(a) => self.eval(&a.0)?.mul(&self.eval(&a.1)?),
            SimpleExpr::Div(a) => self.eval(&a.0)?.div(&self.eval(&a.1)?),
            SimpleExpr::Mod(a) => self.eval(&a.0)?.modulo(&self.eval(&a.1)?),
            SimpleExpr::Pow(a) => self.eval(&a.0)?.pow(&self.eval(&a.1)?),
            SimpleExpr::And(a) => {
                let v = self.eval(&a.0)?.as_bool()? && self.eval(&a.1)?.as_bool()?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            SimpleExpr::Or(a) => {
                let v = self.eval(&a.0)?.as_bool()? || self.eval(&a.1)?.as_bool()?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            SimpleExpr::Not(a) => {
                let v = !self.eval(a)?.as_bool()?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            SimpleExpr::Xor(a) => {
                let v = self.eval(&a.0)?.as_bool()? ^ self.eval(&a.1)?.as_bool()?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            SimpleExpr::Lt(a) => {
                let left = self.eval(&a.0)?;
                let right = self.eval(&a.1)?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o == Ordering::Less)
                    .ok_or_else(|| anyhow!("cannot compare {} < {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            SimpleExpr::Le(a) => {
                let left = self.eval(&a.0)?;
                let right = self.eval(&a.1)?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o != Ordering::Greater)
                    .ok_or_else(|| anyhow!("cannot compare {} ≤ {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            SimpleExpr::Gt(a) => {
                let left = self.eval(&a.0)?;
                let right = self.eval(&a.1)?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o == Ordering::Greater)
                    .ok_or_else(|| anyhow!("cannot compare {} > {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            SimpleExpr::Ge(a) => {
                let left = self.eval(&a.0)?;
                let right = self.eval(&a.1)?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o != Ordering::Less)
                    .ok_or_else(|| anyhow!("cannot compare {} ≥ {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            SimpleExpr::Eq(a) => {
                let left = self.eval(&a.0)?;
                let right = self.eval(&a.1)?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o == Ordering::Equal)
                    .ok_or_else(|| anyhow!("cannot compare {} = {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            SimpleExpr::Ne(a) => {
                let left = self.eval(&a.0)?;
                let right = self.eval(&a.1)?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o != Ordering::Equal)
                    .ok_or_else(|| anyhow!("cannot compare {} ≠ {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use actyx_sdk::NodeId;

    use super::*;
    use quickcheck::{quickcheck, TestResult};
    use spectral::{assert_that, string::StrAssertions};

    fn eval(cx: &Context, s: &str) -> anyhow::Result<String> {
        cx.eval(&s.parse()?).map(|x| x.value().to_string())
    }

    fn eval_bool(cx: &Context, s: &str) -> bool {
        eval(cx, s).unwrap().parse::<bool>().unwrap()
    }

    fn ctx() -> Context<'static> {
        Context::new(EventKey {
            lamport: Default::default(),
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(0.into()),
            offset: Default::default(),
        })
    }

    #[test]
    fn simple() {
        let mut cx = ctx();
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
            err.contains("path .a does not exist in value {\"y\": 42}"),
            "didn’t match: {}",
            err
        );
    }

    #[test]
    fn primitives() {
        let cx = ctx();
        assert_eq!(eval(&cx, "NULL").unwrap(), "null");
        assert_eq!(eval(&cx, "TRUE").unwrap(), "true");
        assert_eq!(eval(&cx, "FALSE").unwrap(), "false");
        assert_eq!(eval(&cx, "1.23").unwrap(), "1.23");
        assert_eq!(eval(&cx, "12345678901234567890").unwrap(), "12345678901234567890");
        assert_eq!(eval(&cx, "''").unwrap(), "\"\"");
        assert_eq!(eval(&cx, "\"\"").unwrap(), "\"\"");
        assert_eq!(eval(&cx, "'hello'").unwrap(), "\"hello\"");
        assert_eq!(eval(&cx, "\"hello\"").unwrap(), "\"hello\"");
        assert_eq!(eval(&cx, r#"'h"ell''o'"#).unwrap(), r#""h\"ell\'o""#);
        assert_eq!(eval(&cx, r#""h""ell'o""#).unwrap(), r#""h\"ell\'o""#);
    }

    #[test]
    fn boolean() {
        let cx = ctx();

        assert_eq!(eval(&cx, "FALSE ∧ FALSE").unwrap(), "false");
        assert_eq!(eval(&cx, "FALSE ∧ TRUE").unwrap(), "false");
        assert_eq!(eval(&cx, "TRUE ∧ FALSE").unwrap(), "false");
        assert_eq!(eval(&cx, "TRUE ∧ TRUE").unwrap(), "true");

        assert_eq!(eval(&cx, "FALSE ∨ FALSE").unwrap(), "false");
        assert_eq!(eval(&cx, "FALSE ∨ TRUE").unwrap(), "true");
        assert_eq!(eval(&cx, "TRUE ∨ FALSE").unwrap(), "true");
        assert_eq!(eval(&cx, "TRUE ∨ TRUE").unwrap(), "true");

        assert_eq!(eval(&cx, "FALSE ⊻ FALSE").unwrap(), "false");
        assert_eq!(eval(&cx, "FALSE ⊻ TRUE").unwrap(), "true");
        assert_eq!(eval(&cx, "TRUE ⊻ FALSE").unwrap(), "true");
        assert_eq!(eval(&cx, "TRUE ⊻ TRUE").unwrap(), "false");

        assert_eq!(eval(&cx, "!FALSE").unwrap(), "true");
        assert_eq!(eval(&cx, "¬TRUE").unwrap(), "false");

        // check short-circuit behaviour
        assert_eq!(eval(&cx, "FALSE & 12").unwrap(), "false");
        assert_eq!(eval(&cx, "TRUE | 12").unwrap(), "true");

        assert_that(&eval(&cx, "NULL & x").unwrap_err().to_string()).contains("null is not a bool");
        assert_that(&eval(&cx, "FALSE | 12").unwrap_err().to_string()).contains("12 is not a bool");
        assert_that(&eval(&cx, "!'a'").unwrap_err().to_string()).contains("\"a\" is not a bool");
    }

    #[test]
    fn compare() {
        let cx = ctx();

        assert_eq!(eval(&cx, "NULL = NULL ∧ NULL ≥ NULL ∧ NULL ≤ NULL").unwrap(), "true");
        assert_eq!(eval(&cx, "NULL ≠ NULL ∨ NULL > NULL ∨ NULL < NULL").unwrap(), "false");

        #[allow(clippy::bool_comparison)]
        fn prop_bool(left: bool, right: bool) -> bool {
            let mut cx = ctx();
            cx.bind("a", cx.value(|b| b.encode_bool(left)));
            cx.bind("b", cx.value(|b| b.encode_bool(right)));
            assert_eq!(eval_bool(&cx, "a < b"), left < right);
            assert_eq!(eval_bool(&cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&cx, "a > b"), left > right);
            assert_eq!(eval_bool(&cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&cx, "a = b"), left == right);
            assert_eq!(eval_bool(&cx, "a ≠ b"), left != right);
            true
        }
        quickcheck(prop_bool as fn(bool, bool) -> bool);

        fn prop_u64(left: u64, right: u64) -> bool {
            let mut cx = ctx();
            cx.bind("a", cx.value(|b| b.encode_u64(left)));
            cx.bind("b", cx.value(|b| b.encode_u64(right)));
            assert_eq!(eval_bool(&cx, "a < b"), left < right);
            assert_eq!(eval_bool(&cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&cx, "a > b"), left > right);
            assert_eq!(eval_bool(&cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&cx, "a = b"), left == right);
            assert_eq!(eval_bool(&cx, "a ≠ b"), left != right);
            true
        }
        quickcheck(prop_u64 as fn(u64, u64) -> bool);

        #[allow(clippy::float_cmp)]
        fn prop_f64(left: f64, right: f64) -> TestResult {
            if left.is_nan() || right.is_nan() {
                return TestResult::discard();
            }
            let mut cx = ctx();
            cx.bind("a", cx.value(|b| b.encode_f64(left)));
            cx.bind("b", cx.value(|b| b.encode_f64(right)));
            assert_eq!(eval_bool(&cx, "a < b"), left < right);
            assert_eq!(eval_bool(&cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&cx, "a > b"), left > right);
            assert_eq!(eval_bool(&cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&cx, "a = b"), left == right);
            assert_eq!(eval_bool(&cx, "a ≠ b"), left != right);
            TestResult::passed()
        }
        quickcheck(prop_f64 as fn(f64, f64) -> TestResult);

        fn prop_str(left: String, right: String) -> bool {
            let mut cx = ctx();
            cx.bind("a", cx.value(|b| b.encode_str(left.as_str())));
            cx.bind("b", cx.value(|b| b.encode_str(right.as_str())));
            assert_eq!(eval_bool(&cx, "a < b"), left < right);
            assert_eq!(eval_bool(&cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&cx, "a > b"), left > right);
            assert_eq!(eval_bool(&cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&cx, "a = b"), left == right);
            assert_eq!(eval_bool(&cx, "a ≠ b"), left != right);
            true
        }
        quickcheck(prop_str as fn(String, String) -> bool);

        assert_that(&eval(&cx, "NULL > 12").unwrap_err().to_string()).contains("cannot compare");
    }

    #[test]
    fn constructors() {
        let cx = ctx();
        assert_eq!(eval(&cx, "([1,'x',NULL]).0").unwrap(), "1");
        assert_eq!(eval(&cx, "([1,'x',NULL]).1").unwrap(), "\"x\"");
        assert_eq!(eval(&cx, "([1,'x',NULL]).2").unwrap(), "null");
        assert_eq!(eval(&cx, "({ one: 1, two: 'x', three: NULL }).one").unwrap(), "1");
        assert_eq!(eval(&cx, "({ one: 1 two: 'x' three: NULL }).two").unwrap(), "\"x\"");
        assert_eq!(eval(&cx, "({ one: 1, two: 'x', three: NULL }).three").unwrap(), "null");

        assert_that(&eval(&cx, "{'x':1}").unwrap_err().to_string()).contains("expected ident");
    }

    #[test]
    fn arithmetic() {
        let cx = ctx();
        assert_eq!(eval(&cx, "1+2").unwrap(), "3");
        assert_eq!(eval(&cx, "1+2*3^2%5").unwrap(), "4");
        assert_eq!(eval(&cx, "1.0+2.0*3.0^2.0%5.0").unwrap(), "4.0");
    }

    #[test]
    fn indexing() {
        let cx = ctx();
        assert_eq!(eval(&cx, "([42]).0").unwrap(), "42");
        assert_eq!(eval(&cx, "([42]).[0]").unwrap(), "42");
        assert_eq!(eval(&cx, "([42]).[1-1]").unwrap(), "42");
    }

    #[test]
    fn cases() {
        let cx = ctx();
        assert_eq!(eval(&cx, "CASE 5 ≤ 5 => 42 CASE TRUE => NULL ENDCASE").unwrap(), "42");
        assert_eq!(eval(&cx, "CASE 5 < 5 => 42 CASE TRUE => NULL ENDCASE").unwrap(), "null");

        assert_that(&eval(&cx, "CASE 1 => 11 ENDCASE").unwrap_err().to_string()).contains("1 is not a bool");
        assert_that(&eval(&cx, "CASE FALSE => 1 ENDCASE").unwrap_err().to_string()).contains("no case matched");
    }
}
