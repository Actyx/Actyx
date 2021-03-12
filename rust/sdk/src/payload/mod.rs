/*
 * Copyright 2020 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use libipld::DagCbor;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

mod opaque;

pub use opaque::Opaque;

/// Compact binary storage of events created when they are received from the Event Service
///
/// see [`Event::extract`](struct.Event.html#method.extract) for supported ways of using the
/// data
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd, DagCbor)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[ipld(repr = "value")]
pub struct Payload(Opaque);

impl Payload {
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
    pub fn empty() -> Payload {
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
}

impl Default for Payload {
    fn default() -> Self {
        Payload::empty()
    }
}

impl Debug for Payload {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.json_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::from_cbor_me;
    use libipld::{cbor::DagCborCodec, codec::Codec};

    #[test]
    fn payload_dag_cbor_roundtrip() -> anyhow::Result<()> {
        let text = "";
        // using JSON value allows CBOR to use known-length array encoding
        let p1: Payload = serde_json::from_value(json!([text]))?;
        let tmp = DagCborCodec.encode(&p1)?;
        let expected = from_cbor_me(
            r#"
81     # array(1)
   60  # text(0)
       # ""
"#,
        )?;
        assert_eq!(tmp, expected);
        let p2: Payload = DagCborCodec.decode(&tmp)?;
        assert_eq!(p1, p2);
        Ok(())
    }
}
