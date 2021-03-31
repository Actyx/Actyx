use actyxos_sdk::{
    language::{Index, Number},
    service::EventResponse,
    Event, EventKey, Payload,
};
use anyhow::{anyhow, Result};
use cbor_data::{CborBuilder, CborOwned, CborValue, Encoder, WithOutput};
use std::{
    cell::RefCell,
    fmt::{self, Display, Formatter},
};

thread_local! {
    static SCRATCH: RefCell<Vec<u8>> = RefCell::new(vec![]);
}

#[derive(Debug, PartialEq, Clone)]
pub struct Value {
    pub sort_key: EventKey,
    value: CborOwned, // should later become InternedHash<[u8]>
}

impl From<Event<Payload>> for Value {
    fn from(ev: Event<Payload>) -> Self {
        Self {
            sort_key: ev.key,
            value: CborOwned::trusting(ev.payload.as_bytes()),
        }
    }
}

impl From<Value> for EventResponse<Payload> {
    fn from(value: Value) -> EventResponse<Payload> {
        EventResponse {
            lamport: value.sort_key.lamport,
            stream: value.sort_key.stream,
            offset: value.sort_key.offset,
            timestamp: Default::default(),
            tags: Default::default(),
            payload: Payload::from_bytes(value.value.as_ref()),
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

    pub fn key(&self) -> EventKey {
        self.sort_key
    }

    pub fn value(&self) -> CborValue {
        self.value.value().unwrap()
    }

    pub fn index<'a>(&'a self, path: &[Index]) -> Option<CborValue> {
        // FIXME this needs to be made nice and easy by adding an Index trait to cbor-data
        let path = path
            .iter()
            .map(|i| match i {
                Index::Ident(s) => s.clone(),
                Index::Number(n) => format!("{}", n),
            })
            .collect::<Vec<_>>()
            .join(".");
        self.value.index(&*path)
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

    pub fn number(&self, n: Number) -> Value {
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

    pub fn gt(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(Value::new(self.sort_key, |b| b.encode_bool(lhs > rhs)))
    }

    pub fn lt(&self, other: &Value) -> Result<Value> {
        let lhs = self.as_number()?;
        let rhs = other.as_number()?;
        Ok(Value::new(self.sort_key, |b| b.encode_bool(lhs < rhs)))
    }
}
