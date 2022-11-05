use crate::{error::RuntimeError, query::Query, value::Value};
use actyx_sdk::{
    language::{BinOp, Ind, Index, Num, SimpleExpr, TagAtom, TagExpr},
    service::{EventMeta, Order},
    OffsetMap, Tag, Timestamp,
};
use anyhow::{anyhow, bail, ensure};
use cbor_data::{
    value::{self as cbor_value, Precision},
    CborBuilder, CborOwned, CborValue, Encoder, PathElement, WithOutput, Writer,
};
use futures::{future::BoxFuture, FutureExt};
use parking_lot::Mutex;
use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
    sync::Arc,
};
use swarm::event_store_ref::EventStoreRef;

pub struct RootContext {
    scratch: Mutex<Vec<u8>>,
    store: EventStoreRef,
    from_offsets_excluding: OffsetMap,
    to_offsets_including: OffsetMap,
    order: Order,
}

impl RootContext {
    pub fn child(&self) -> Context<'_> {
        Context {
            root: self,
            parent: None,
            bindings: BTreeMap::new(),
            order: self.order,
        }
    }
}

pub struct Context<'a> {
    root: &'a RootContext,
    parent: Option<&'a Context<'a>>,
    bindings: BTreeMap<String, anyhow::Result<Value>>,
    pub order: Order,
}

impl<'a> Context<'a> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(store: EventStoreRef) -> RootContext {
        Self::root(Order::Asc, store, OffsetMap::empty(), OffsetMap::empty())
    }

    pub fn root(
        order: Order,
        store: EventStoreRef,
        from_offsets_excluding: OffsetMap,
        to_offsets_including: OffsetMap,
    ) -> RootContext {
        RootContext {
            scratch: Mutex::new(Vec::new()),
            store,
            from_offsets_excluding,
            to_offsets_including,
            order,
        }
    }

    pub fn child(&'a self) -> Self {
        Self {
            root: self.root,
            parent: Some(self),
            bindings: BTreeMap::new(),
            order: self.order,
        }
    }

    pub fn child_with_order(&'a self, order: Order) -> Self {
        Self {
            root: self.root,
            parent: Some(self),
            bindings: BTreeMap::new(),
            order,
        }
    }

    pub fn store(&self) -> &EventStoreRef {
        &self.root.store
    }

    pub fn from_offsets_excluding(&self) -> &OffsetMap {
        &self.root.from_offsets_excluding
    }

    pub fn to_offsets_including(&self) -> &OffsetMap {
        &self.root.to_offsets_including
    }

    pub fn bind(&mut self, name: impl Into<String>, value: Value) {
        self.bindings.insert(name.into(), Ok(value));
    }

    pub fn bind_placeholder(&mut self, name: String, value: anyhow::Result<Value>) {
        self.bindings.insert(name, value);
    }

    pub fn lookup_opt(&self, name: &str) -> Option<&anyhow::Result<Value>> {
        self.bindings
            .get(name)
            .or_else(|| self.parent.and_then(|c| c.lookup_opt(name)))
    }

    pub fn lookup(&self, name: &str) -> anyhow::Result<Value> {
        self.lookup_opt(name).map_or_else(
            || Err(RuntimeError::NotBound(name.to_owned()).into()),
            |v| match v {
                Ok(v) => Ok(v.clone()),
                Err(e) => Err(anyhow!("{}", e)),
            },
        )
    }

    pub fn remove(&mut self, name: &str) -> anyhow::Result<Value> {
        self.bindings
            .remove(name)
            .unwrap_or_else(|| Err(RuntimeError::NotBound(name.to_owned()).into()))
    }

    pub fn mk_cbor(&self, f: impl FnOnce(CborBuilder<WithOutput>) -> CborOwned) -> CborOwned {
        let mut buf = self.root.scratch.try_lock();
        let mut fallback = Vec::new();
        let buf = buf.as_mut().map(|b| b.as_mut()).unwrap_or(&mut fallback);
        f(CborBuilder::with_scratch_space(buf))
    }

    pub fn number(&self, n: &Num) -> CborOwned {
        match n {
            Num::Decimal(d) => self.mk_cbor(|b| b.encode_f64(*d)),
            Num::Natural(n) => self.mk_cbor(|b| b.encode_u64(*n)),
        }
    }

    pub fn eval_from<'b, 'c: 'b>(&'b self, expr: &'c TagExpr) -> BoxFuture<'b, anyhow::Result<Cow<'c, TagExpr>>> {
        async move {
            match expr {
                TagExpr::Or(x) => {
                    let left = self.eval_from(&x.0).await?;
                    let right = self.eval_from(&x.1).await?;
                    if let (Cow::Borrowed(_), Cow::Borrowed(_)) = (&left, &right) {
                        Ok(Cow::Borrowed(expr))
                    } else {
                        Ok(Cow::Owned(TagExpr::Or(Arc::new((
                            left.into_owned(),
                            right.into_owned(),
                        )))))
                    }
                }
                TagExpr::And(x) => {
                    let left = self.eval_from(&x.0).await?;
                    let right = self.eval_from(&x.1).await?;
                    if let (Cow::Borrowed(_), Cow::Borrowed(_)) = (&left, &right) {
                        Ok(Cow::Borrowed(expr))
                    } else {
                        Ok(Cow::Owned(TagExpr::And(Arc::new((
                            left.into_owned(),
                            right.into_owned(),
                        )))))
                    }
                }
                TagExpr::Atom(a) => match a {
                    TagAtom::Interpolation(s) => {
                        let mut buf = String::new();
                        for e in s {
                            buf.push_str(&self.eval(e).await?.print());
                        }
                        Ok(Cow::Owned(TagExpr::Atom(TagAtom::Tag(Tag::try_from(&*buf)?))))
                    }
                    _ => Ok(Cow::Borrowed(expr)),
                },
            }
        }
        .boxed()
    }

    pub fn eval<'c>(&'c self, expr: &'c SimpleExpr) -> BoxFuture<'c, anyhow::Result<Value>> {
        async move {
            match expr {
                SimpleExpr::Variable(v) => self.lookup(v),
                SimpleExpr::Indexing(Ind { head, tail }) => {
                    let v = self.eval(head).await?;
                    let meta = v.meta().clone();
                    let mut v = v.cbor();
                    for i in tail.iter() {
                        v =
                            match i {
                                Index::String(s) => v
                                    .index_borrowed([PathElement::String(Cow::Borrowed(s))])
                                    .ok_or_else(|| RuntimeError::NotFound {
                                        index: s.clone(),
                                        in_value: (&v.decode()).into(),
                                    })?,
                                Index::Number(n) => v.index_borrowed([PathElement::Number(*n)]).ok_or_else(|| {
                                    RuntimeError::NotFound {
                                        index: format!("{:?}", n),
                                        in_value: (&v.decode()).into(),
                                    }
                                })?,
                                Index::Expr(e) => {
                                    let idx = self.eval(e).await?;
                                    match idx.value() {
                                        CborValue::Number(cbor_value::Number::Int(i)) => v
                                            .index_borrowed([PathElement::Number(i.try_into()?)])
                                            .ok_or_else(|| RuntimeError::NotFound {
                                                index: format!("{:?}", i),
                                                in_value: (&v.decode()).into(),
                                            })?,
                                        CborValue::Str(s) => v
                                            .index_borrowed([PathElement::String(s.clone())])
                                            .ok_or_else(|| RuntimeError::NotFound {
                                                index: s.into_owned(),
                                                in_value: (&v.decode()).into(),
                                            })?,
                                        _ => return Err(RuntimeError::NotAnIndex(idx.to_string()).into()),
                                    }
                                }
                            };
                    }
                    Ok(Value::new_meta(v.to_owned(), meta))
                }
                SimpleExpr::Number(n) => Ok(Value::synthetic(self.number(n))),
                SimpleExpr::String(s) => Ok(Value::synthetic(self.mk_cbor(|b| b.encode_str(s)))),
                SimpleExpr::Interpolation(s) => {
                    let mut buf = String::new();
                    let mut meta = EventMeta::Synthetic;
                    for e in s {
                        let v = self.eval(e).await?;
                        meta += v.meta();
                        buf.push_str(&v.print());
                    }
                    Ok(Value::new_meta(self.mk_cbor(|b| b.encode_str(buf)), meta))
                }
                SimpleExpr::Object(a) => {
                    let mut v = BTreeMap::new();
                    let mut meta = EventMeta::Synthetic;
                    for (n, e) in a.props.iter() {
                        let key = match n {
                            Index::String(s) => self.mk_cbor(|b| b.encode_str(s)),
                            Index::Number(n) => self.mk_cbor(|b| b.encode_u64(*n)),
                            Index::Expr(e) => {
                                let v = self.eval(e).await?;
                                meta += v.meta();
                                v.cbor().to_owned()
                            }
                        };
                        let val = self.eval(e).await?;
                        meta += val.meta();
                        v.insert(key, val);
                    }
                    Ok(Value::new_meta(
                        self.mk_cbor(|b| {
                            b.encode_dict(|b| {
                                for (name, item) in v.iter() {
                                    b.with_cbor_key(
                                        |b| b.write_trusting(name.as_slice()),
                                        |b| b.write_trusting(item.as_slice()),
                                    );
                                }
                            })
                        }),
                        meta,
                    ))
                }
                SimpleExpr::Array(a) => {
                    let mut v = vec![];
                    let mut meta = EventMeta::Synthetic;
                    for e in a.items.iter() {
                        let val = self.eval(e).await?;
                        meta += val.meta();
                        if e.spread {
                            if let Ok(items) = val.as_array() {
                                v.extend(items);
                            } else {
                                return Err(RuntimeError::TypeErrorSpread(val.kind()).into());
                            }
                        } else {
                            v.push(val);
                        }
                    }
                    Ok(Value::new_meta(
                        self.mk_cbor(|b| {
                            b.encode_array(|b| {
                                for item in v.iter() {
                                    b.write_trusting(item.as_slice());
                                }
                            })
                        }),
                        meta,
                    ))
                }
                SimpleExpr::Null => Ok(Value::synthetic(self.mk_cbor(|b| b.write_null(None)))),
                SimpleExpr::Bool(f) => Ok(Value::synthetic(self.mk_cbor(|b| b.write_bool(*f, None)))),
                SimpleExpr::Cases(v) => {
                    for (pred, expr) in v.iter() {
                        let pred = self.eval(pred).await.and_then(|v| v.as_bool());
                        if pred.unwrap_or(false) {
                            return self.eval(expr).await;
                        }
                    }
                    Err(anyhow!("no case matched"))
                }
                SimpleExpr::Not(a) => {
                    let v = self.eval(a).await?;
                    let meta = v.meta().clone();
                    let v = !v.as_bool()?;
                    Ok(Value::new_meta(self.mk_cbor(|b| b.encode_bool(v)), meta))
                }
                SimpleExpr::BinOp(b) => self.bin_op(&b.1, b.0, &b.2).await,
                SimpleExpr::AggrOp(a) => bail!("internal error, unreplaced AGGREGATE operator: {}", a.0.as_str()),
                SimpleExpr::FuncCall(f) => match f.name.as_str() {
                    "IsDefined" => {
                        ensure!(
                            f.args.len() == 1,
                            "wrong number of arguments: 'IsDefined' takes 1 argument but {} were provided",
                            f.args.len()
                        );
                        let defined = self.eval(&f.args[0]).await.is_ok();
                        Ok(Value::synthetic(self.mk_cbor(|b| b.encode_bool(defined))))
                    }
                    _ => Err(anyhow!("undefined function '{}'", f.name)),
                },
                SimpleExpr::SubQuery(q) => {
                    let arr = Query::eval(q, self).await?;
                    let meta = arr.iter().fold(EventMeta::Synthetic, |mut meta, v| {
                        meta += v.meta();
                        meta
                    });
                    Ok(Value::new_meta(
                        self.mk_cbor(|b| {
                            b.encode_array(|b| {
                                for v in arr {
                                    b.write_trusting(v.as_slice());
                                }
                            })
                        }),
                        meta,
                    ))
                }
                SimpleExpr::KeyVar(var) => {
                    let v = self
                        .lookup_opt(var.as_ref())
                        .ok_or_else(|| RuntimeError::NotBound(var.to_string()))?;
                    match v {
                        Ok(v) => {
                            let meta = v.meta().clone();
                            match &meta {
                                EventMeta::Range { from_key, to_key, .. } => Ok(Value::new_meta(
                                    self.mk_cbor(|b| {
                                        b.encode_array(|b| {
                                            b.encode_array(|b| {
                                                b.encode_u64(from_key.lamport.into());
                                                b.encode_bytes(from_key.stream.node_id.as_ref());
                                                b.encode_u64(from_key.stream.stream_nr.into());
                                            });
                                            b.encode_array(|b| {
                                                b.encode_u64(to_key.lamport.into());
                                                b.encode_bytes(to_key.stream.node_id.as_ref());
                                                b.encode_u64(to_key.stream.stream_nr.into());
                                            });
                                        })
                                    }),
                                    meta,
                                )),
                                EventMeta::Synthetic => {
                                    Ok(Value::new_meta(self.mk_cbor(|b| b.encode_array(|_| {})), meta))
                                }
                                EventMeta::Event { key, .. } => Ok(Value::new_meta(
                                    self.mk_cbor(|b| {
                                        b.encode_array(|b| {
                                            b.encode_array(|b| {
                                                b.encode_u64(key.lamport.into());
                                                b.encode_bytes(key.stream.node_id.as_ref());
                                                b.encode_u64(key.stream.stream_nr.into());
                                            });
                                        })
                                    }),
                                    meta,
                                )),
                            }
                        }
                        Err(_) => Err(RuntimeError::NotBound(var.to_string()).into()),
                    }
                }
                SimpleExpr::KeyLiteral(key) => Ok(Value::synthetic(self.mk_cbor(|b| {
                    b.encode_array(|b| {
                        b.encode_array(|b| {
                            b.encode_u64(key.lamport.into());
                            b.encode_bytes(key.stream.node_id.as_ref());
                            b.encode_u64(key.stream.stream_nr.into());
                        });
                    })
                }))),
                SimpleExpr::TimeVar(var) => {
                    let v = self
                        .lookup_opt(var.as_ref())
                        .ok_or_else(|| RuntimeError::NotBound(var.to_string()))?;
                    match v {
                        Ok(v) => {
                            let m = v.meta().clone();
                            match &m {
                                EventMeta::Range { from_time, to_time, .. } => {
                                    let from = time_to_cbor(*from_time);
                                    let to = time_to_cbor(*to_time);
                                    Ok(Value::new_meta(
                                        self.mk_cbor(|b| {
                                            b.encode_array(|b| {
                                                b.encode_timestamp(from, Precision::Micros);
                                                b.encode_timestamp(to, Precision::Micros);
                                            })
                                        }),
                                        m,
                                    ))
                                }
                                EventMeta::Synthetic => {
                                    Ok(Value::new_meta(self.mk_cbor(|b| b.encode_array(|_| {})), m))
                                }
                                EventMeta::Event { meta, .. } => {
                                    let timestamp = time_to_cbor(meta.timestamp);
                                    Ok(Value::new_meta(
                                        self.mk_cbor(|b| {
                                            b.encode_array(|b| {
                                                b.encode_timestamp(timestamp, Precision::Micros);
                                            })
                                        }),
                                        m,
                                    ))
                                }
                            }
                        }
                        Err(_) => Err(RuntimeError::NotBound(var.to_string()).into()),
                    }
                }
                SimpleExpr::TimeLiteral(t) => Ok(Value::synthetic(
                    self.mk_cbor(|b| b.encode_timestamp(time_to_cbor(*t), Precision::Micros)),
                )),
                SimpleExpr::Tags(var) => {
                    let v = self
                        .lookup_opt(var.as_ref())
                        .ok_or_else(|| RuntimeError::NotBound(var.to_string()))?;
                    match v {
                        Ok(v) => {
                            let m = v.meta().clone();
                            match &m {
                                EventMeta::Range { .. } => {
                                    Ok(Value::new_meta(self.mk_cbor(|b| b.encode_array(|_| {})), m))
                                }
                                EventMeta::Synthetic => {
                                    Ok(Value::new_meta(self.mk_cbor(|b| b.encode_array(|_| {})), m))
                                }
                                EventMeta::Event { meta, .. } => Ok(Value::new_meta(
                                    self.mk_cbor(|b| {
                                        b.encode_array(|b| {
                                            for t in &meta.tags {
                                                b.encode_str(t.as_ref());
                                            }
                                        })
                                    }),
                                    m,
                                )),
                            }
                        }
                        Err(_) => Err(RuntimeError::NotBound(var.to_string()).into()),
                    }
                }
                SimpleExpr::App(var) => {
                    let v = self
                        .lookup_opt(var.as_ref())
                        .ok_or_else(|| RuntimeError::NotBound(var.to_string()))?;
                    match v {
                        Ok(v) => {
                            let m = v.meta().clone();
                            match &m {
                                EventMeta::Range { .. } => {
                                    Ok(Value::new_meta(self.mk_cbor(|b| b.encode_array(|_| {})), m))
                                }
                                EventMeta::Synthetic => {
                                    Ok(Value::new_meta(self.mk_cbor(|b| b.encode_array(|_| {})), m))
                                }
                                EventMeta::Event { meta, .. } => Ok(Value::new_meta(
                                    self.mk_cbor(|b| {
                                        b.encode_array(|b| {
                                            b.encode_str(meta.app_id.as_str());
                                        })
                                    }),
                                    m,
                                )),
                            }
                        }
                        Err(_) => Err(RuntimeError::NotBound(var.to_string()).into()),
                    }
                }
            }
        }
        .boxed()
    }

    async fn bin_op<'c>(&'c self, l: &'c SimpleExpr, op: BinOp, r: &'c SimpleExpr) -> anyhow::Result<Value> {
        if op == BinOp::Alt {
            return match self.eval(l).await {
                Ok(v) => Ok(v),
                Err(_) => self.eval(r).await,
            };
        }
        let left = self.eval(l).await?;
        let right = self.eval(r).await?;
        let value = match op {
            BinOp::Add => {
                let n = left.as_number()?.add(&right.as_number()?)?;
                self.number(&n)
            }
            BinOp::Sub => {
                let n = left.as_number()?.sub(&right.as_number()?)?;
                self.number(&n)
            }
            BinOp::Mul => {
                let n = left.as_number()?.mul(&right.as_number()?)?;
                self.number(&n)
            }
            BinOp::Div => {
                let n = left.as_number()?.div(&right.as_number()?)?;
                self.number(&n)
            }
            BinOp::Mod => {
                let n = left.as_number()?.modulo(&right.as_number()?)?;
                self.number(&n)
            }
            BinOp::Pow => {
                let n = left.as_number()?.pow(&right.as_number()?)?;
                self.number(&n)
            }
            BinOp::And => {
                let v = left.as_bool()? && right.as_bool()?;
                self.mk_cbor(|b| b.encode_bool(v))
            }
            BinOp::Or => {
                let v = left.as_bool()? || right.as_bool()?;
                self.mk_cbor(|b| b.encode_bool(v))
            }
            BinOp::Xor => {
                let v = left.as_bool()? ^ right.as_bool()?;
                self.mk_cbor(|b| b.encode_bool(v))
            }
            BinOp::Lt => {
                let v = (left.partial_cmp(&right)).map(|o| o == Ordering::Less).ok_or_else(|| {
                    RuntimeError::TypeErrorBinOp {
                        op: BinOp::Lt,
                        left: left.kind(),
                        right: right.kind(),
                    }
                })?;
                self.mk_cbor(|b| b.encode_bool(v))
            }
            BinOp::Le => {
                let v = (left.partial_cmp(&right))
                    .map(|o| o != Ordering::Greater)
                    .ok_or_else(|| RuntimeError::TypeErrorBinOp {
                        op: BinOp::Le,
                        left: left.kind(),
                        right: right.kind(),
                    })?;
                self.mk_cbor(|b| b.encode_bool(v))
            }
            BinOp::Gt => {
                let v = (left.partial_cmp(&right))
                    .map(|o| o == Ordering::Greater)
                    .ok_or_else(|| RuntimeError::TypeErrorBinOp {
                        op: BinOp::Gt,
                        left: left.kind(),
                        right: right.kind(),
                    })?;
                self.mk_cbor(|b| b.encode_bool(v))
            }
            BinOp::Ge => {
                let v = (left.partial_cmp(&right)).map(|o| o != Ordering::Less).ok_or_else(|| {
                    RuntimeError::TypeErrorBinOp {
                        op: BinOp::Ge,
                        left: left.kind(),
                        right: right.kind(),
                    }
                })?;
                self.mk_cbor(|b| b.encode_bool(v))
            }
            BinOp::Eq => {
                let v = (left.partial_cmp(&right))
                    .map(|o| o == Ordering::Equal)
                    .ok_or_else(|| RuntimeError::TypeErrorBinOp {
                        op: BinOp::Eq,
                        left: left.kind(),
                        right: right.kind(),
                    })?;
                self.mk_cbor(|b| b.encode_bool(v))
            }
            BinOp::Ne => {
                let v = (left.partial_cmp(&right))
                    .map(|o| o != Ordering::Equal)
                    .ok_or_else(|| RuntimeError::TypeErrorBinOp {
                        op: BinOp::Ne,
                        left: left.kind(),
                        right: right.kind(),
                    })?;
                self.mk_cbor(|b| b.encode_bool(v))
            }
            BinOp::Alt => unreachable!(),
        };
        let mut meta = left.meta().clone();
        meta += right.meta();
        Ok(Value::new_meta(value, meta))
    }
}

fn time_to_cbor(t: Timestamp) -> cbor_value::Timestamp {
    let mut micros = t.as_i64() % 1_000_000;
    if micros < 0 {
        micros += 1_000_000
    }
    cbor_value::Timestamp::new(t.as_i64() / 1_000_000, micros as u32 * 1000, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use quickcheck::{quickcheck, TestResult};
    use spectral::{assert_that, string::StrAssertions};

    async fn eval(cx: &mut Context<'_>, s: &str) -> anyhow::Result<String> {
        cx.eval(&s.parse()?).await.map(|x| x.cbor().to_string())
    }

    fn eval_bool(cx: &mut Context<'_>, s: &str) -> bool {
        block_on(eval(cx, s)).unwrap().parse::<bool>().unwrap()
    }

    fn mk_store() -> EventStoreRef {
        EventStoreRef::new(|_x| Err(swarm::event_store_ref::Error::Aborted))
    }
    fn ctx() -> RootContext {
        Context::new(mk_store())
    }

    #[tokio::test]
    async fn simple() {
        let cx = ctx();
        let mut cx = cx.child();
        cx.bind(
            "x",
            Value::synthetic(cx.mk_cbor(|b| {
                b.encode_dict(|b| {
                    b.with_key("y", |b| b.encode_u64(42));
                })
            })),
        );

        assert_eq!(eval(&mut cx, "5+2.1+x.y").await.unwrap(), "49.1");

        assert_eq!(eval(&mut cx, "x").await.unwrap(), "{\"y\": 42}");

        let err = eval(&mut cx, "5+x").await.unwrap_err().to_string();
        assert!(err.contains("`OBJECT` is not of type Number"), "didn’t match: {}", err);

        let err = eval(&mut cx, "y").await.unwrap_err().to_string();
        assert!(err.contains("variable `y` is not bound"), "didn’t match: {}", err);

        let err = eval(&mut cx, "x.a").await.unwrap_err().to_string();
        assert!(
            err.contains("property `a` not found in Object"),
            "didn’t match: {}",
            err
        );
    }

    #[tokio::test]
    async fn primitives() {
        let cx = ctx();
        let mut cx = cx.child();
        assert_eq!(eval(&mut cx, "NULL").await.unwrap(), "null");
        assert_eq!(eval(&mut cx, "TRUE").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "FALSE").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "1.23").await.unwrap(), "1.23");
        assert_eq!(
            eval(&mut cx, "12345678901234567890").await.unwrap(),
            "12345678901234567890"
        );
        assert_eq!(eval(&mut cx, "''").await.unwrap(), "\"\"");
        assert_eq!(eval(&mut cx, "\"\"").await.unwrap(), "\"\"");
        assert_eq!(eval(&mut cx, "'hello'").await.unwrap(), "\"hello\"");
        assert_eq!(eval(&mut cx, "\"hello\"").await.unwrap(), "\"hello\"");
        assert_eq!(eval(&mut cx, r#"'h"ell''o'"#).await.unwrap(), r#""h\"ell\'o""#);
        assert_eq!(eval(&mut cx, r#""h""ell'o""#).await.unwrap(), r#""h\"ell\'o""#);
    }

    #[tokio::test]
    async fn boolean() {
        let cx = ctx();
        let mut cx = cx.child();

        assert_eq!(eval(&mut cx, "FALSE ∧ FALSE").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "FALSE ∧ TRUE").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "TRUE ∧ FALSE").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "TRUE ∧ TRUE").await.unwrap(), "true");

        assert_eq!(eval(&mut cx, "FALSE ∨ FALSE").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "FALSE ∨ TRUE").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "TRUE ∨ FALSE").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "TRUE ∨ TRUE").await.unwrap(), "true");

        assert_eq!(eval(&mut cx, "FALSE ⊻ FALSE").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "FALSE ⊻ TRUE").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "TRUE ⊻ FALSE").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "TRUE ⊻ TRUE").await.unwrap(), "false");

        assert_eq!(eval(&mut cx, "!FALSE").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "¬TRUE").await.unwrap(), "false");

        // check short-circuit behaviour
        assert_eq!(eval(&mut cx, "FALSE & 12").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "TRUE | 12").await.unwrap(), "true");

        assert_that(&eval(&mut cx, "NULL & 'x'").await.unwrap_err().to_string()).contains("null is not a bool");
        assert_that(&eval(&mut cx, "FALSE | 12").await.unwrap_err().to_string()).contains("12 is not a bool");
        assert_that(&eval(&mut cx, "!'a'").await.unwrap_err().to_string()).contains("\"a\" is not a bool");
    }

    #[tokio::test]
    async fn compare() {
        let cx = ctx();
        let mut cx = cx.child();

        assert_eq!(
            eval(&mut cx, "NULL = NULL ∧ NULL ≥ NULL ∧ NULL ≤ NULL").await.unwrap(),
            "true"
        );
        assert_eq!(
            eval(&mut cx, "NULL ≠ NULL ∨ NULL > NULL ∨ NULL < NULL").await.unwrap(),
            "false"
        );

        #[allow(clippy::bool_comparison)]
        fn prop_bool(left: bool, right: bool) -> bool {
            let cx = ctx();
            let mut cx = cx.child();
            cx.bind("a", Value::synthetic(cx.mk_cbor(|b| b.encode_bool(left))));
            cx.bind("b", Value::synthetic(cx.mk_cbor(|b| b.encode_bool(right))));
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
            let cx = ctx();
            let mut cx = cx.child();
            cx.bind("a", Value::synthetic(cx.mk_cbor(|b| b.encode_u64(left))));
            cx.bind("b", Value::synthetic(cx.mk_cbor(|b| b.encode_u64(right))));
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
            let cx = ctx();
            let mut cx = cx.child();
            cx.bind("a", Value::synthetic(cx.mk_cbor(|b| b.encode_f64(left))));
            cx.bind("b", Value::synthetic(cx.mk_cbor(|b| b.encode_f64(right))));
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
            let cx = ctx();
            let mut cx = cx.child();
            cx.bind("a", Value::synthetic(cx.mk_cbor(|b| b.encode_str(left.as_str()))));
            cx.bind("b", Value::synthetic(cx.mk_cbor(|b| b.encode_str(right.as_str()))));
            assert_eq!(eval_bool(&mut cx, "a < b"), left < right);
            assert_eq!(eval_bool(&mut cx, "a ≤ b"), left <= right);
            assert_eq!(eval_bool(&mut cx, "a > b"), left > right);
            assert_eq!(eval_bool(&mut cx, "a ≥ b"), left >= right);
            assert_eq!(eval_bool(&mut cx, "a = b"), left == right);
            assert_eq!(eval_bool(&mut cx, "a ≠ b"), left != right);
            true
        }
        quickcheck(prop_str as fn(String, String) -> bool);

        assert_eq!(
            &eval(&mut cx, "NULL > 12").await.unwrap_err().to_string(),
            "binary operation > cannot be applied to Null and Number"
        );
    }

    #[tokio::test]
    async fn constructors() {
        let cx = ctx();
        let mut cx = cx.child();
        assert_eq!(eval(&mut cx, "([1,'x',NULL])[0]").await.unwrap(), "1");
        assert_eq!(eval(&mut cx, "([1,'x',NULL])[1]").await.unwrap(), "\"x\"");
        assert_eq!(eval(&mut cx, "([1,'x',NULL])[2]").await.unwrap(), "null");
        assert_eq!(
            eval(&mut cx, "({ one: 1, ['two']: 'x', [('three')]: NULL, [4]: TRUE }).one")
                .await
                .unwrap(),
            "1"
        );
        assert_eq!(
            eval(&mut cx, "({ one: 1 ['two']: 'x' [('three')]: NULL, [4]: TRUE }).two")
                .await
                .unwrap(),
            "\"x\""
        );
        assert_eq!(
            eval(
                &mut cx,
                "({ one: 1, ['two']: 'x', [('three')]: NULL, [4]: TRUE }).three"
            )
            .await
            .unwrap(),
            "null"
        );
        assert_eq!(
            eval(&mut cx, "({ one: 1, ['two']: 'x', [('three')]: NULL, [4]: TRUE })[4]")
                .await
                .unwrap(),
            "true"
        );

        assert_that(&eval(&mut cx, "{'x':1}").await.unwrap_err().to_string()).contains("expected ident");
    }

    #[tokio::test]
    async fn arithmetic() {
        let cx = ctx();
        let mut cx = cx.child();
        assert_eq!(eval(&mut cx, "1+2").await.unwrap(), "3");
        assert_eq!(eval(&mut cx, "1+2*3^2%5").await.unwrap(), "4");
        assert_eq!(eval(&mut cx, "1.0+2.0*3.0^2.0%5.0").await.unwrap(), "4.0");

        assert_that(
            &eval(&mut cx, "12345678901234567890 + 12345678901234567890")
                .await
                .unwrap_err()
                .to_string(),
        )
        .contains("integer overflow");
        assert_that(&eval(&mut cx, "10.0 ^ 400").await.unwrap_err().to_string()).contains("floating-point overflow");
        assert_that(&eval(&mut cx, "10.0 / 0").await.unwrap_err().to_string()).contains("floating-point overflow");
        assert_that(&eval(&mut cx, "0.0 / 0").await.unwrap_err().to_string()).contains("not a number");
    }

    #[tokio::test]
    async fn indexing() {
        let cx = ctx();
        let mut cx = cx.child();
        assert_eq!(eval(&mut cx, "([42])[0]").await.unwrap(), "42");
        assert_eq!(eval(&mut cx, "([42])[1-1]").await.unwrap(), "42");
        assert_eq!(eval(&mut cx, "({x:12}).x").await.unwrap(), "12");
        assert_eq!(eval(&mut cx, "({x:12})['x']").await.unwrap(), "12");
        assert_eq!(eval(&mut cx, "({x:12})[('x')]").await.unwrap(), "12");
    }

    #[tokio::test]
    async fn cases() {
        let cx = ctx();
        let mut cx = cx.child();
        assert_eq!(
            eval(&mut cx, "CASE 5 ≤ 5 => 42 CASE TRUE => NULL ENDCASE")
                .await
                .unwrap(),
            "42"
        );
        assert_eq!(
            eval(&mut cx, "CASE 5 < 5 => 42 CASE TRUE => NULL ENDCASE")
                .await
                .unwrap(),
            "null"
        );
        assert_eq!(
            eval(&mut cx, "CASE 'a' => 'b' CASE TRUE => 'c' ENDCASE").await.unwrap(),
            "\"c\""
        );
        assert_eq!(
            eval(&mut cx, "CASE a => 'b' CASE TRUE => 'c' ENDCASE").await.unwrap(),
            "\"c\""
        );

        assert_that(&eval(&mut cx, "CASE FALSE => 1 ENDCASE").await.unwrap_err().to_string())
            .contains("no case matched");
    }

    #[tokio::test]
    async fn alternative() {
        let cx = ctx();
        let mut cx = cx.child();
        assert_eq!(eval(&mut cx, "5 ?? 6").await.unwrap(), "5");
        assert_eq!(eval(&mut cx, "(5).a ?? 6").await.unwrap(), "6");
        assert_eq!(eval(&mut cx, "NULL ?? 1").await.unwrap(), "null");
    }

    #[tokio::test]
    async fn builtin_functions() {
        let cx = ctx();
        let mut cx = cx.child();

        assert_eq!(eval(&mut cx, "IsDefined(1)").await.unwrap(), "true");
        assert_eq!(eval(&mut cx, "IsDefined(1 + '')").await.unwrap(), "false");
        assert_eq!(eval(&mut cx, "IsDefined(1 + '' ?? FALSE)").await.unwrap(), "true");
        assert_that(&eval(&mut cx, "IsDefined()").await.unwrap_err().to_string()).contains("wrong number of arguments");
        assert_that(&eval(&mut cx, "IsDefined(1, 2)").await.unwrap_err().to_string())
            .contains("wrong number of arguments");
    }
}
