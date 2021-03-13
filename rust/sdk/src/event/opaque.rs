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
use crate::types::ArcVal;
use libipld::{
    cbor::DagCborCodec,
    codec::{Decode, Encode},
    raw_value::RawValue,
};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;

/// A ref-counted slice of memory holding a compact binary representation of an event payload
///
/// see [`Event::extract`](struct.Event.html#method.extract) for supported ways of using the
/// data
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct Opaque(ArcVal<[u8]>);

impl Opaque {
    pub fn new(bytes: Arc<[u8]>) -> Self {
        Opaque(bytes.into())
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        Opaque(ArcVal::clone_from_unsized(bytes))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Rough estimate of the in memory size of an opaque value
    pub fn rough_size(&self) -> usize {
        self.len() + 16
    }
}

impl AsRef<[u8]> for Opaque {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Serialize for Opaque {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut deserializer = serde_cbor::Deserializer::from_slice(&self.0);
        serde_transcode::transcode(&mut deserializer, serializer)
    }
}

impl<'de> Deserialize<'de> for Opaque {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let res = Vec::new();
        let mut serializer = serde_cbor::Serializer::new(res);
        serde_transcode::transcode(deserializer, &mut serializer).map_err(D::Error::custom)?;
        let res = serializer.into_inner();
        Ok(Opaque(ArcVal::from_boxed(res.into())))
    }
}

impl Encode<DagCborCodec> for Opaque {
    fn encode<W: std::io::Write>(&self, _c: DagCborCodec, w: &mut W) -> anyhow::Result<()> {
        // we know that an opaque contains cbor, so we just write it
        Ok(w.write_all(self.as_ref())?)
    }
}

impl Decode<DagCborCodec> for Opaque {
    fn decode<R: std::io::Read + std::io::Seek>(c: DagCborCodec, r: &mut R) -> anyhow::Result<Self> {
        let tmp = RawValue::<DagCborCodec>::decode(c, r)?;
        Ok(Self(ArcVal::from_boxed(tmp.into())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::from_cbor_me;
    use libipld::codec::Codec;
    use serde_json::json;

    #[test]
    fn opaque_dag_cbor_roundtrip() -> anyhow::Result<()> {
        let text = "";
        // using JSON value allows CBOR to use known-length array encoding
        let o1: Opaque = serde_json::from_value(json!([text]))?;
        let tmp = DagCborCodec.encode(&o1)?;
        let expected = from_cbor_me(
            r#"
81     # array(1)
   60  # text(0)
       # ""
"#,
        )?;
        assert_eq!(tmp, expected);
        let o2: Opaque = DagCborCodec.decode(&tmp)?;
        assert_eq!(o1, o2);
        Ok(())
    }

    #[test]
    fn u128_is_f64() {
        // this test is to ensure that serde_json does not convert long integers to u128, which CBOR does not support
        let text = format!("{}", u128::max_value());
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(value.is_f64());
    }

    #[test]
    fn i128_is_f64() {
        // this test is to ensure that serde_json does not convert long signed integers to u128, which CBOR does not support
        let text = format!("{}", i128::min_value());
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(value.is_f64());
    }
}
