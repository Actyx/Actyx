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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn opaque_json_roundtrip() {
        let text = "";
        // using JSON value allows CBOR to use known-length array encoding
        let o1: Opaque = serde_json::from_value(json!([text])).unwrap();
        let j = serde_json::to_string(&o1).unwrap();
        // if we don’t parse the JSON first then CBOR will use indefinite length encoding
        let v = serde_json::from_str(&j).unwrap();
        let o2: Opaque = serde_json::from_value(v).unwrap();
        assert_eq!(o1, o2);
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
