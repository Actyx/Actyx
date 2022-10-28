use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

use super::Opaque;
use cbor_data::cbor_via;

/// Compact binary storage of events created when they are received
///
/// see [`Event::extract`](struct.Event.html#method.extract) for supported ways of using the
/// data
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct Payload(Opaque);

impl Payload {
    pub fn from_slice(bytes: &[u8]) -> Self {
        Self(Opaque::new(bytes.into()))
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn from_json_str(s: &str) -> Result<Payload, String> {
        serde_json::from_str(s).map_err(|e| format!("{}", e))
    }

    /// Construct a new Payload from the supplied serializable value.
    pub fn compact<T: Serialize>(t: &T) -> Result<Payload, serde_cbor::Error> {
        serde_cbor::to_vec(t).map(|bytes| Payload(Opaque::new(bytes.into())))
    }

    /// Try to lift the desired type from this Payload’s bytes.
    pub fn extract<'a, T: Deserialize<'a>>(&'a self) -> Result<T, serde_cbor::Error> {
        serde_cbor::from_slice(self.0.as_ref())
    }

    /// Transform into a generic JSON structure that you can then traverse or query.
    pub fn json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }

    /// Printable representation of this stored object as JSON — the stored Payload
    /// bytes are encoded in the CBOR binary format.
    pub fn json_string(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    /// Construct a Payload consisting only of the `null` value.
    pub fn null() -> Payload {
        Payload(serde_json::from_str("null").unwrap())
    }

    /// Rough estimate of the in memory size of the contained opaque value
    pub fn rough_size(&self) -> usize {
        self.0.rough_size()
    }

    /// Only to be used from tests, since it has bad performance due to a serde bug/issue
    pub fn from_json_value(v: serde_json::Value) -> Result<Payload, String> {
        // weirdly we have to canonicalize this!
        let text = serde_json::to_string(&v).unwrap();
        Payload::from_json_str(&text)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(Opaque::from_bytes(bytes))
    }
}

impl Default for Payload {
    fn default() -> Self {
        Payload::null()
    }
}

impl Debug for Payload {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.json_string())
    }
}

cbor_via!(Payload => Opaque: |p| -> p.0.clone(), |o| -> Ok(Payload(o)));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::from_cbor_me;
    use cbor_data::{
        codec::{ReadCbor, WriteCbor},
        CborBuilder,
    };

    #[test]
    fn payload_dag_cbor_roundtrip() -> anyhow::Result<()> {
        let text = "";
        // using JSON value allows CBOR to use known-length array encoding
        let p1: Payload = serde_json::from_value(json!([text]))?;
        let tmp = p1.write_cbor(CborBuilder::default());
        let expected = from_cbor_me(
            r#"
 81     # array(1)
    60  # text(0)
        # ""
 "#,
        )?;
        assert_eq!(tmp.as_slice(), expected);
        let p2 = Payload::read_cbor(&*tmp)?;
        assert_eq!(p1, p2);
        Ok(())
    }
}
