use crate::{
    operation::AggrState,
    value::{Value, ValueKind},
};
use actyx_sdk::language::{BinOp, Ind, Index, Num, SimpleExpr, SortKey};
use anyhow::{anyhow, bail};
use cbor_data::{CborBuilder, CborOwned, Encoder, WithOutput, Writer};
use std::{cmp::Ordering, collections::BTreeMap};

pub struct Context<'a> {
    sort_key: SortKey,
    bindings: BTreeMap<String, Value>,
    parent: Option<&'a Context<'a>>,
    aggregation: Option<&'a mut AggrState>,
}

impl<'a> Context<'a> {
    pub fn new(sort_key: SortKey) -> Self {
        Self {
            sort_key,
            bindings: BTreeMap::new(),
            parent: None,
            aggregation: None,
        }
    }

    pub fn child(&'a self) -> Self {
        Self {
            sort_key: self.sort_key,
            bindings: BTreeMap::new(),
            parent: Some(self),
            aggregation: None,
        }
    }

    #[cfg(test)]
    pub fn incr(&mut self) {
        self.sort_key.lamport = self.sort_key.lamport.incr();
    }

    pub fn value(&self, f: impl FnOnce(CborBuilder<WithOutput>) -> CborOwned) -> Value {
        Value::new(self.sort_key, f)
    }

    pub fn number(&self, n: &Num) -> Value {
        match n {
            Num::Decimal(d) => Value::new(self.sort_key, |b| b.encode_f64(*d)),
            Num::Natural(n) => Value::new(self.sort_key, |b| b.encode_u64(*n)),
        }
    }

    pub fn bind(&mut self, name: impl Into<String>, value: Value) {
        self.bindings.insert(name.into(), value);
    }

    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.bindings
            .get(name)
            .or_else(|| self.parent.and_then(|c| c.lookup(name)))
    }

    pub fn bind_aggregation(&mut self, state: &'a mut AggrState) {
        self.aggregation = Some(state);
    }

    pub fn eval(&mut self, expr: &SimpleExpr) -> anyhow::Result<Value> {
        match expr {
            SimpleExpr::Variable(v) => {
                let v = self.lookup(v).ok_or_else(|| anyhow!("variable '{}' is not bound", v))?;
                Ok(self.value(|b| b.write_trusting(v.as_slice())))
            }
            SimpleExpr::Indexing(Ind { head, tail }) => {
                let mut v = self.eval(head)?;
                for i in tail.iter() {
                    v = match i {
                        Index::String(s) => v.index(s)?,
                        Index::Number(n) => v.index(&format!("{}", n))?,
                        Index::Expr(e) => {
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
                Num::Decimal(d) => Ok(self.value(|b| b.encode_f64(*d))),
                Num::Natural(n) => Ok(self.value(|b| b.encode_u64(*n))),
            },
            SimpleExpr::String(s) => Ok(self.value(|b| b.encode_str(s))),
            SimpleExpr::Object(a) => {
                let v = a
                    .props
                    .iter()
                    // TODO don’t evaluate overwritten properties
                    .map(|(n, e)| {
                        let key = match n {
                            Index::String(s) => s.clone(),
                            Index::Number(n) => n.to_string(),
                            Index::Expr(e) => {
                                let k = self.eval(e)?;
                                k.as_str()
                                    .map(|s| s.to_owned())
                                    .or_else(|_| k.as_number().map(|n| n.to_string()))?
                            }
                        };
                        Ok((key, self.eval(e)?))
                    })
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
                    let pred = self.eval(pred).and_then(|v| v.as_bool());
                    if pred.unwrap_or(false) {
                        return self.eval(expr);
                    }
                }
                Err(anyhow!("no case matched"))
            }
            SimpleExpr::Not(a) => {
                let v = !self.eval(a)?.as_bool()?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            SimpleExpr::BinOp(b) => self.bin_op(&b.1, b.0, &b.2),
            SimpleExpr::AggrOp(a) => {
                let aggr = self.aggregation.take().ok_or_else(|| anyhow!("no aggregation state"))?;
                let v = aggr
                    .get_mut(a)
                    .ok_or_else(|| anyhow!("no aggregation result for {}({})", a.0.as_str(), a.1))?
                    .flush(self);
                self.aggregation.replace(aggr);
                v
            }
        }
    }

    fn bin_op(&mut self, l: &SimpleExpr, op: BinOp, r: &SimpleExpr) -> anyhow::Result<Value> {
        match op {
            BinOp::Add => self.eval(l)?.add(&self.eval(r)?),
            BinOp::Sub => self.eval(l)?.sub(&self.eval(r)?),
            BinOp::Mul => self.eval(l)?.mul(&self.eval(r)?),
            BinOp::Div => self.eval(l)?.div(&self.eval(r)?),
            BinOp::Mod => self.eval(l)?.modulo(&self.eval(r)?),
            BinOp::Pow => self.eval(l)?.pow(&self.eval(r)?),
            BinOp::And => {
                let v = self.eval(l)?.as_bool()? && self.eval(r)?.as_bool()?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Or => {
                let v = self.eval(l)?.as_bool()? || self.eval(r)?.as_bool()?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Xor => {
                let v = self.eval(l)?.as_bool()? ^ self.eval(r)?.as_bool()?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Lt => {
                let left = self.eval(l)?;
                let right = self.eval(r)?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o == Ordering::Less)
                    .ok_or_else(|| anyhow!("cannot compare {} < {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Le => {
                let left = self.eval(l)?;
                let right = self.eval(r)?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o != Ordering::Greater)
                    .ok_or_else(|| anyhow!("cannot compare {} ≤ {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Gt => {
                let left = self.eval(l)?;
                let right = self.eval(r)?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o == Ordering::Greater)
                    .ok_or_else(|| anyhow!("cannot compare {} > {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Ge => {
                let left = self.eval(l)?;
                let right = self.eval(r)?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o != Ordering::Less)
                    .ok_or_else(|| anyhow!("cannot compare {} ≥ {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Eq => {
                let left = self.eval(l)?;
                let right = self.eval(r)?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o == Ordering::Equal)
                    .ok_or_else(|| anyhow!("cannot compare {} = {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Ne => {
                let left = self.eval(l)?;
                let right = self.eval(r)?;
                let v = (left.partial_cmp(&right))
                    .map(|o| o != Ordering::Equal)
                    .ok_or_else(|| anyhow!("cannot compare {} ≠ {}", left, right))?;
                Ok(self.value(|b| b.encode_bool(v)))
            }
            BinOp::Alt => self.eval(l).or_else(|_| self.eval(r)),
        }
    }
}

#[cfg(test)]
mod tests {
    use actyx_sdk::NodeId;

    use super::*;
    use quickcheck::{quickcheck, TestResult};
    use spectral::{assert_that, string::StrAssertions};

    fn eval(cx: &mut Context, s: &str) -> anyhow::Result<String> {
        cx.eval(&s.parse()?).map(|x| x.value().to_string())
    }

    fn eval_bool(cx: &mut Context, s: &str) -> bool {
        eval(cx, s).unwrap().parse::<bool>().unwrap()
    }

    fn ctx() -> Context<'static> {
        Context::new(SortKey {
            lamport: Default::default(),
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(0.into()),
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

        assert_eq!(eval(&mut cx, "5+2.1+x.y").unwrap(), "49.1");

        assert_eq!(eval(&mut cx, "x").unwrap(), "{\"y\": 42}");

        let err = eval(&mut cx, "5+x").unwrap_err().to_string();
        assert!(err.contains("{\"y\": 42} is not a number"), "didn’t match: {}", err);

        let err = eval(&mut cx, "y").unwrap_err().to_string();
        assert!(err.contains("variable 'y' is not bound"), "didn’t match: {}", err);

        let err = eval(&mut cx, "x.a").unwrap_err().to_string();
        assert!(
            err.contains("path .a does not exist in value {\"y\": 42}"),
            "didn’t match: {}",
            err
        );
    }

    #[test]
    fn primitives() {
        let mut cx = ctx();
        assert_eq!(eval(&mut cx, "NULL").unwrap(), "null");
        assert_eq!(eval(&mut cx, "TRUE").unwrap(), "true");
        assert_eq!(eval(&mut cx, "FALSE").unwrap(), "false");
        assert_eq!(eval(&mut cx, "1.23").unwrap(), "1.23");
        assert_eq!(eval(&mut cx, "12345678901234567890").unwrap(), "12345678901234567890");
        assert_eq!(eval(&mut cx, "''").unwrap(), "\"\"");
        assert_eq!(eval(&mut cx, "\"\"").unwrap(), "\"\"");
        assert_eq!(eval(&mut cx, "'hello'").unwrap(), "\"hello\"");
        assert_eq!(eval(&mut cx, "\"hello\"").unwrap(), "\"hello\"");
        assert_eq!(eval(&mut cx, r#"'h"ell''o'"#).unwrap(), r#""h\"ell\'o""#);
        assert_eq!(eval(&mut cx, r#""h""ell'o""#).unwrap(), r#""h\"ell\'o""#);
    }

    #[test]
    fn boolean() {
        let mut cx = ctx();

        assert_eq!(eval(&mut cx, "FALSE ∧ FALSE").unwrap(), "false");
        assert_eq!(eval(&mut cx, "FALSE ∧ TRUE").unwrap(), "false");
        assert_eq!(eval(&mut cx, "TRUE ∧ FALSE").unwrap(), "false");
        assert_eq!(eval(&mut cx, "TRUE ∧ TRUE").unwrap(), "true");

        assert_eq!(eval(&mut cx, "FALSE ∨ FALSE").unwrap(), "false");
        assert_eq!(eval(&mut cx, "FALSE ∨ TRUE").unwrap(), "true");
        assert_eq!(eval(&mut cx, "TRUE ∨ FALSE").unwrap(), "true");
        assert_eq!(eval(&mut cx, "TRUE ∨ TRUE").unwrap(), "true");

        assert_eq!(eval(&mut cx, "FALSE ⊻ FALSE").unwrap(), "false");
        assert_eq!(eval(&mut cx, "FALSE ⊻ TRUE").unwrap(), "true");
        assert_eq!(eval(&mut cx, "TRUE ⊻ FALSE").unwrap(), "true");
        assert_eq!(eval(&mut cx, "TRUE ⊻ TRUE").unwrap(), "false");

        assert_eq!(eval(&mut cx, "!FALSE").unwrap(), "true");
        assert_eq!(eval(&mut cx, "¬TRUE").unwrap(), "false");

        // check short-circuit behaviour
        assert_eq!(eval(&mut cx, "FALSE & 12").unwrap(), "false");
        assert_eq!(eval(&mut cx, "TRUE | 12").unwrap(), "true");

        assert_that(&eval(&mut cx, "NULL & x").unwrap_err().to_string()).contains("null is not a bool");
        assert_that(&eval(&mut cx, "FALSE | 12").unwrap_err().to_string()).contains("12 is not a bool");
        assert_that(&eval(&mut cx, "!'a'").unwrap_err().to_string()).contains("\"a\" is not a bool");
    }

    #[test]
    fn compare() {
        let mut cx = ctx();

        assert_eq!(
            eval(&mut cx, "NULL = NULL ∧ NULL ≥ NULL ∧ NULL ≤ NULL").unwrap(),
            "true"
        );
        assert_eq!(
            eval(&mut cx, "NULL ≠ NULL ∨ NULL > NULL ∨ NULL < NULL").unwrap(),
            "false"
        );

        #[allow(clippy::bool_comparison)]
        fn prop_bool(left: bool, right: bool) -> bool {
            let mut cx = ctx();
            cx.bind("a", cx.value(|b| b.encode_bool(left)));
            cx.bind("b", cx.value(|b| b.encode_bool(right)));
            assert_eq!(eval_bool(&mut cx, "a < b"), left < right);
            assert_eq!(eval_bool(&mut cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&mut cx, "a > b"), left > right);
            assert_eq!(eval_bool(&mut cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&mut cx, "a = b"), left == right);
            assert_eq!(eval_bool(&mut cx, "a ≠ b"), left != right);
            true
        }
        quickcheck(prop_bool as fn(bool, bool) -> bool);

        fn prop_u64(left: u64, right: u64) -> bool {
            let mut cx = ctx();
            cx.bind("a", cx.value(|b| b.encode_u64(left)));
            cx.bind("b", cx.value(|b| b.encode_u64(right)));
            assert_eq!(eval_bool(&mut cx, "a < b"), left < right);
            assert_eq!(eval_bool(&mut cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&mut cx, "a > b"), left > right);
            assert_eq!(eval_bool(&mut cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&mut cx, "a = b"), left == right);
            assert_eq!(eval_bool(&mut cx, "a ≠ b"), left != right);
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
            assert_eq!(eval_bool(&mut cx, "a < b"), left < right);
            assert_eq!(eval_bool(&mut cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&mut cx, "a > b"), left > right);
            assert_eq!(eval_bool(&mut cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&mut cx, "a = b"), left == right);
            assert_eq!(eval_bool(&mut cx, "a ≠ b"), left != right);
            TestResult::passed()
        }
        quickcheck(prop_f64 as fn(f64, f64) -> TestResult);

        fn prop_str(left: String, right: String) -> bool {
            let mut cx = ctx();
            cx.bind("a", cx.value(|b| b.encode_str(left.as_str())));
            cx.bind("b", cx.value(|b| b.encode_str(right.as_str())));
            assert_eq!(eval_bool(&mut cx, "a < b"), left < right);
            assert_eq!(eval_bool(&mut cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&mut cx, "a > b"), left > right);
            assert_eq!(eval_bool(&mut cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&mut cx, "a = b"), left == right);
            assert_eq!(eval_bool(&mut cx, "a ≠ b"), left != right);
            true
        }
        quickcheck(prop_str as fn(String, String) -> bool);

        assert_that(&eval(&mut cx, "NULL > 12").unwrap_err().to_string()).contains("cannot compare");
    }

    #[test]
    fn constructors() {
        let mut cx = ctx();
        assert_eq!(eval(&mut cx, "([1,'x',NULL])[0]").unwrap(), "1");
        assert_eq!(eval(&mut cx, "([1,'x',NULL])[1]").unwrap(), "\"x\"");
        assert_eq!(eval(&mut cx, "([1,'x',NULL])[2]").unwrap(), "null");
        assert_eq!(
            eval(&mut cx, "({ one: 1, ['two']: 'x', [('three')]: NULL, [4]: TRUE }).one").unwrap(),
            "1"
        );
        assert_eq!(
            eval(&mut cx, "({ one: 1 ['two']: 'x' [('three')]: NULL, [4]: TRUE }).two").unwrap(),
            "\"x\""
        );
        assert_eq!(
            eval(
                &mut cx,
                "({ one: 1, ['two']: 'x', [('three')]: NULL, [4]: TRUE }).three"
            )
            .unwrap(),
            "null"
        );
        assert_eq!(
            eval(&mut cx, "({ one: 1, ['two']: 'x', [('three')]: NULL, [4]: TRUE })[4]").unwrap(),
            "true"
        );

        assert_that(&eval(&mut cx, "{'x':1}").unwrap_err().to_string()).contains("expected ident");
    }

    #[test]
    fn arithmetic() {
        let mut cx = ctx();
        assert_eq!(eval(&mut cx, "1+2").unwrap(), "3");
        assert_eq!(eval(&mut cx, "1+2*3^2%5").unwrap(), "4");
        assert_eq!(eval(&mut cx, "1.0+2.0*3.0^2.0%5.0").unwrap(), "4.0");

        assert_that(
            &eval(&mut cx, "12345678901234567890 + 12345678901234567890")
                .unwrap_err()
                .to_string(),
        )
        .contains("integer overflow");
        assert_that(&eval(&mut cx, "10.0 ^ 400").unwrap_err().to_string()).contains("floating-point overflow");
        assert_that(&eval(&mut cx, "10.0 / 0").unwrap_err().to_string()).contains("floating-point overflow");
        assert_that(&eval(&mut cx, "0.0 / 0").unwrap_err().to_string()).contains("not a number");
    }

    #[test]
    fn indexing() {
        let mut cx = ctx();
        assert_eq!(eval(&mut cx, "([42])[0]").unwrap(), "42");
        assert_eq!(eval(&mut cx, "([42])[1-1]").unwrap(), "42");
        assert_eq!(eval(&mut cx, "({x:12}).x").unwrap(), "12");
        assert_eq!(eval(&mut cx, "({x:12})['x']").unwrap(), "12");
        assert_eq!(eval(&mut cx, "({x:12})[('x')]").unwrap(), "12");
    }

    #[test]
    fn cases() {
        let mut cx = ctx();
        assert_eq!(
            eval(&mut cx, "CASE 5 ≤ 5 => 42 CASE TRUE => NULL ENDCASE").unwrap(),
            "42"
        );
        assert_eq!(
            eval(&mut cx, "CASE 5 < 5 => 42 CASE TRUE => NULL ENDCASE").unwrap(),
            "null"
        );
        assert_eq!(
            eval(&mut cx, "CASE 'a' => 'b' CASE TRUE => 'c' ENDCASE").unwrap(),
            "\"c\""
        );
        assert_eq!(
            eval(&mut cx, "CASE a => 'b' CASE TRUE => 'c' ENDCASE").unwrap(),
            "\"c\""
        );

        assert_that(&eval(&mut cx, "CASE FALSE => 1 ENDCASE").unwrap_err().to_string()).contains("no case matched");
    }

    #[test]
    fn alternative() {
        let mut cx = ctx();
        assert_eq!(eval(&mut cx, "5 // 6").unwrap(), "5");
        assert_eq!(eval(&mut cx, "(5).a // 6").unwrap(), "6");
    }
}
