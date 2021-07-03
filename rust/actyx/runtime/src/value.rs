use actyx_sdk::{language::Number, EventKey, Payload};
use anyhow::{anyhow, Result};
use cbor_data::{Cbor, CborBuilder, CborOwned, CborValue, Encoder, WithOutput};
use std::{
    cell::RefCell,
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    iter::once,
};

thread_local! {
    static SCRATCH: RefCell<Vec<u8>> = RefCell::new(vec![]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Null,
    Bool,
    Number,
    String,
    Object,
    Array,
    Other,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Value {
    sort_key: EventKey,
    value: CborOwned, // should later become InternedHash<[u8]>
}

impl From<(EventKey, Payload)> for Value {
    fn from(event: (EventKey, Payload)) -> Self {
        let (key, payload) = event;
        Self {
            sort_key: key,
            value: CborOwned::trusting(payload.as_bytes()),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}@{}: {}",
            u64::from(self.sort_key.lamport),
            self.sort_key.stream,
            self.value.to_string()
        )
    }
}

impl Value {
    pub fn new(sort_key: EventKey, f: impl FnOnce(CborBuilder<WithOutput>) -> CborOwned) -> Self {
        Self {
            sort_key,
            value: SCRATCH.with(|v| f(CborBuilder::with_scratch_space(&mut (*v).borrow_mut()))),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.value.as_slice()
    }

    pub fn key(&self) -> EventKey {
        self.sort_key
    }

    pub fn value(&self) -> CborValue<'_> {
        self.value.value().unwrap()
    }

    pub fn cbor(&self) -> Cbor<'_> {
        self.value.borrow()
    }

    pub fn index(&self, s: &str) -> anyhow::Result<Self> {
        self.value
            .index_iter(once(s))
            .map(|cbor| Self {
                sort_key: self.sort_key,
                value: CborOwned::trusting(cbor.bytes),
            })
            .ok_or_else(|| anyhow!("path .{} does not exist in value {}", s, self.value()))
    }

    pub fn kind(&self) -> ValueKind {
        match self.value().kind {
            cbor_data::ValueKind::Pos(_) => ValueKind::Number,
            cbor_data::ValueKind::Neg(_) => ValueKind::Number,
            cbor_data::ValueKind::Float(_) => ValueKind::Number,
            cbor_data::ValueKind::Str(_) => ValueKind::String,
            cbor_data::ValueKind::Bytes(_) => ValueKind::Other,
            cbor_data::ValueKind::Bool(_) => ValueKind::Bool,
            cbor_data::ValueKind::Null => ValueKind::Null,
            cbor_data::ValueKind::Undefined => ValueKind::Other,
            cbor_data::ValueKind::Simple(_) => ValueKind::Other,
            cbor_data::ValueKind::Array => ValueKind::Array,
            cbor_data::ValueKind::Dict => ValueKind::Object,
        }
    }

    pub fn payload(&self) -> Payload {
        Payload::from_bytes(self.value.as_ref())
    }

    pub fn as_number(&self) -> Result<Number> {
        if let Some(b) = self.value().as_u64() {
            Ok(Number::Natural(b))
        } else if let Some(f) = self.value().as_f64() {
            Ok(Number::Decimal(f))
        } else {
            Err(anyhow!("{} is not a number", self))
        }
    }

    pub fn as_bool(&self) -> Result<bool> {
        self.value().as_bool().ok_or_else(|| anyhow!("{} is not a bool", self))
    }

    pub fn as_str(&self) -> Result<&str> {
        self.value().as_str().ok_or_else(|| anyhow!("{} is not a string", self))
    }

    fn number(&self, n: Number) -> Value {
        match n {
            Number::Decimal(d) => Value::new(self.sort_key, |b| b.encode_f64(d)),
            Number::Natural(n) => Value::new(self.sort_key, |b| b.encode_u64(n)),
        }
    }

    pub fn add(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(self.number(lhs.add(&rhs)))
    }

    pub fn sub(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(self.number(lhs.sub(&rhs)))
    }

    pub fn mul(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(self.number(lhs.mul(&rhs)))
    }

    pub fn div(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(self.number(lhs.div(&rhs)))
    }

    pub fn modulo(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(self.number(lhs.modulo(&rhs)))
    }

    pub fn pow(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(self.number(lhs.pow(&rhs)))
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
            ValueKind::String => self.as_str().ok()?.partial_cmp(other.as_str().ok()?),
            _ => None,
        }
    }
}
