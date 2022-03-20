use crate::{error::RuntimeError, eval::Context};
use actyx_sdk::{
    language::{Num, SortKey},
    EventKey, Payload,
};
use anyhow::{anyhow, Result};
use cbor_data::{Cbor, CborBuilder, CborOwned, CborValue, Encoder, WithOutput, Writer};
use chrono::{DateTime, SecondsFormat};
use derive_more::Display;
use std::{
    borrow::{Borrow, Cow},
    cell::RefCell,
    cmp::Ordering,
    convert::{TryFrom, TryInto},
    fmt::{self, Display, Formatter},
};

thread_local! {
    static SCRATCH: RefCell<Vec<u8>> = RefCell::new(vec![]);
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Null,
    Bool,
    Timestamp,
    Number,
    String,
    Bytes,
    Object,
    Array,
    Other,
}

impl<'a> From<&CborValue<'a>> for ValueKind {
    fn from(v: &CborValue<'a>) -> Self {
        match v {
            CborValue::Array(_) => ValueKind::Array,
            CborValue::Dict(_) => ValueKind::Object,
            CborValue::Undefined => ValueKind::Object,
            CborValue::Null => ValueKind::Null,
            CborValue::Bool(_) => ValueKind::Bool,
            CborValue::Number(_) => ValueKind::Number,
            CborValue::Timestamp(_) => ValueKind::Timestamp,
            CborValue::Str(_) => ValueKind::String,
            CborValue::Bytes(_) => ValueKind::Bytes,
            CborValue::Invalid => ValueKind::Other,
            CborValue::Unknown => ValueKind::Other,
        }
    }
}

impl<'a> From<CborValue<'a>> for ValueKind {
    fn from(v: CborValue<'a>) -> Self {
        (&v).into()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Value {
    sort_key: SortKey,
    value: CborOwned, // should later become InternedHash<[u8]>
}

impl From<(EventKey, Payload)> for Value {
    fn from(event: (EventKey, Payload)) -> Self {
        let (key, payload) = event;
        Self {
            sort_key: key.into(),
            value: CborOwned::unchecked(payload.as_bytes()),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}: {}",
            u64::from(self.sort_key.lamport),
            self.sort_key.stream,
            self.value
        )
    }
}

impl Value {
    pub fn new(sort_key: SortKey, f: impl FnOnce(CborBuilder<WithOutput>) -> CborOwned) -> Self {
        Self {
            sort_key,
            value: SCRATCH.with(|v| f(CborBuilder::with_scratch_space(&mut (*v).borrow_mut()))),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.value.as_slice()
    }

    pub fn key(&self) -> SortKey {
        self.sort_key
    }

    pub fn value(&self) -> CborValue<'_> {
        self.value.decode()
    }

    pub fn kind(&self) -> ValueKind {
        self.value.decode().into()
    }

    pub fn cbor(&self) -> &Cbor {
        self.value.borrow()
    }

    pub fn payload(&self) -> Payload {
        Payload::from_bytes(self.value.as_ref())
    }

    pub fn as_number(&self) -> Result<Num> {
        match self.value() {
            CborValue::Number(n) => match n {
                cbor_data::value::Number::Int(i) => Ok(Num::Natural(i.try_into()?)),
                cbor_data::value::Number::IEEE754(f) => Ok(Num::Decimal(f)),
                cbor_data::value::Number::Decimal(_) => Err(RuntimeError::NotSupported("BigDecimal".to_owned()).into()),
                cbor_data::value::Number::Float(_) => Err(RuntimeError::NotSupported("BigFloat".to_owned()).into()),
            },
            _ => Err(RuntimeError::TypeError {
                value: self.print(),
                expected: ValueKind::Number,
            }
            .into()),
        }
    }

    pub fn as_bool(&self) -> Result<bool> {
        self.value().as_bool().ok_or_else(|| anyhow!("{} is not a bool", self))
    }

    pub fn as_str(&self) -> Result<Cow<'_, str>> {
        self.value().to_str().ok_or_else(|| anyhow!("{} is not a string", self))
    }

    pub fn as_array(&self, cx: &Context) -> Result<Vec<Value>> {
        Ok(self
            .value()
            .to_array()
            .ok_or_else(|| RuntimeError::TypeError {
                value: self.print(),
                expected: ValueKind::Array,
            })?
            .into_iter()
            .map(|cbor| cx.value(|b| b.write_trusting(cbor.as_slice())))
            .collect())
    }

    pub fn print(&self) -> String {
        match self.value() {
            CborValue::Array(_) => "ARRAY".to_owned(),
            CborValue::Dict(_) => "OBJECT".to_owned(),
            CborValue::Undefined => "UNDEFINED".to_owned(),
            CborValue::Null => "NULL".to_owned(),
            CborValue::Bool(b) => if b { "TRUE" } else { "FALSE" }.to_owned(),
            CborValue::Number(_) => self.cbor().to_string(),
            CborValue::Timestamp(t) => DateTime::try_from(t)
                .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Micros, true))
                .unwrap_or_else(|_| "INVALID_TIMESTAMP".to_owned()),
            CborValue::Str(s) => s.into_owned(),
            CborValue::Bytes(_) => self.cbor().to_string(),
            CborValue::Invalid => "INVALID".to_owned(),
            CborValue::Unknown => "UNKNWON".to_owned(),
        }
    }

    fn number(&self, n: Num) -> Value {
        match n {
            Num::Decimal(d) => Value::new(self.sort_key, |b| b.encode_f64(d)),
            Num::Natural(n) => Value::new(self.sort_key, |b| b.encode_u64(n)),
        }
    }

    pub fn add(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(self.number(lhs.add(&rhs)?))
    }

    pub fn sub(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(self.number(lhs.sub(&rhs)?))
    }

    pub fn mul(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(self.number(lhs.mul(&rhs)?))
    }

    pub fn div(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(self.number(lhs.div(&rhs)?))
    }

    pub fn modulo(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(self.number(lhs.modulo(&rhs)?))
    }

    pub fn pow(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(self.number(lhs.pow(&rhs)?))
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let left = self.kind();
        let right = other.kind();
        if left != right {
            return None;
        }
        match left {
            ValueKind::Null => Some(Ordering::Equal),
            ValueKind::Bool => self.as_bool().ok()?.partial_cmp(&other.as_bool().ok()?),
            ValueKind::Number => self.as_number().ok()?.partial_cmp(&other.as_number().ok()?),
            ValueKind::String => self.as_str().ok()?.as_ref().partial_cmp(other.as_str().ok()?.as_ref()),
            _ => None,
        }
    }
}
