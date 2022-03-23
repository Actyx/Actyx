use crate::error::RuntimeError;
use actyx_sdk::{
    language::Num,
    service::{EventMeta, EventResponse},
    Event, EventKey, Payload,
};
use anyhow::{anyhow, Result};
use cbor_data::{Cbor, CborOwned, CborValue};
use chrono::{DateTime, Local, SecondsFormat};
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
    meta: EventMeta,
    value: CborOwned, // should later become InternedHash<[u8]>
}

impl From<Event<Payload>> for Value {
    fn from(event: Event<Payload>) -> Self {
        Self {
            meta: EventMeta::Event {
                key: event.key,
                meta: event.meta,
            },
            value: CborOwned::unchecked(event.payload.as_bytes()),
        }
    }
}
impl From<EventResponse<Payload>> for Value {
    fn from(ev: EventResponse<Payload>) -> Self {
        Self {
            meta: ev.meta,
            value: CborOwned::unchecked(ev.payload.as_bytes()),
        }
    }
}
impl From<Value> for EventResponse<Payload> {
    fn from(ev: Value) -> Self {
        let payload = ev.payload();
        Self { meta: ev.meta, payload }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.meta {
            EventMeta::Range {
                from_key,
                to_key,
                from_time,
                to_time,
            } => write!(
                f,
                "{}/{}@{} - {}/{}@{}: {}",
                u64::from(from_key.lamport),
                from_key.stream.abbreviate(),
                DateTime::from(from_time)
                    .with_timezone(&Local)
                    .to_rfc3339_opts(SecondsFormat::Micros, false),
                u64::from(to_key.lamport),
                to_key.stream.abbreviate(),
                DateTime::from(to_time)
                    .with_timezone(&Local)
                    .to_rfc3339_opts(SecondsFormat::Micros, false),
                self.value
            ),
            EventMeta::Synthetic => write!(f, "synthetic: {}", self.value),
            EventMeta::Event { key, ref meta } => write!(
                f,
                "{}/{}@{}: {}",
                u64::from(key.lamport),
                key.stream.abbreviate(),
                DateTime::from(meta.timestamp)
                    .with_timezone(&Local)
                    .to_rfc3339_opts(SecondsFormat::Micros, false),
                self.value
            ),
        }
    }
}

impl Value {
    pub fn synthetic(value: CborOwned) -> Self {
        Self {
            meta: EventMeta::Synthetic,
            value,
        }
    }

    pub fn new_meta(value: CborOwned, meta: EventMeta) -> Self {
        Self { value, meta }
    }

    pub fn meta(&self) -> &EventMeta {
        &self.meta
    }

    pub fn min_key(&self) -> EventKey {
        match self.meta {
            EventMeta::Range { from_key, .. } => from_key,
            EventMeta::Synthetic => EventKey::ZERO,
            EventMeta::Event { key, .. } => key,
        }
    }

    pub fn max_key(&self) -> EventKey {
        match self.meta {
            EventMeta::Range { to_key, .. } => to_key,
            EventMeta::Synthetic => EventKey::ZERO,
            EventMeta::Event { key, .. } => key,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.value.as_slice()
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

    pub fn as_array(&self) -> Result<Vec<Value>> {
        Ok(self
            .value()
            .to_array()
            .ok_or_else(|| RuntimeError::TypeError {
                value: self.print(),
                expected: ValueKind::Array,
            })?
            .into_iter()
            .map(|cbor| Self {
                meta: self.meta().clone(),
                value: cbor.into_owned(),
            })
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
