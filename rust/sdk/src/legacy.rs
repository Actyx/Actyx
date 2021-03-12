/*
 * Copyright 2021 Actyx AG
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
use std::{
    convert::TryFrom,
    fmt::{self, Debug, Display, Formatter},
    str::FromStr,
};

use anyhow::anyhow;
use derive_more::Display;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use crate::{scalar::nonempty_string, tags::TagSet};

#[derive(Debug, Display, PartialEq)]
pub enum ParseError {
    #[display(fmt = "SourceId was longer than maximum")]
    SourceIdTooLong,
    #[display(fmt = "Empty string is not permissible for SourceId")]
    EmptySourceId,
    #[display(fmt = "Empty string is not permissible for Semantics")]
    EmptySemantics,
    #[display(fmt = "Empty string is not permissible for FishName")]
    EmptyFishName,
    #[display(fmt = "Empty string is not permissible for Tag")]
    EmptyTag,
    #[display(fmt = "Empty string is not permissible for AppId")]
    EmptyAppId,
}
impl std::error::Error for ParseError {}

/// Macro for constructing a [`Semantics`](event/struct.Semantics.html) literal.
///
/// This is how it works:
/// ```no_run
/// use actyxos_sdk::{semantics, legacy::Semantics};
/// let semantics: Semantics = semantics!("abc");
/// ```
/// This does not compile:
/// ```compile_fail
/// use actyxos_sdk::{semantics, legacy::Semantics};
/// let semantics: Semantics = semantics!("");
/// ```
#[macro_export]
macro_rules! semantics {
    ($lit:tt) => {{
        #[allow(dead_code)]
        type X = $crate::assert_len!($lit, 1..);
        use ::std::convert::TryFrom;
        $crate::legacy::Semantics::try_from($lit).unwrap()
    }};
}

/// Macro for constructing a [`FishName`](event/struct.FishName.html) literal.
///
/// This is how it works:
/// ```no_run
/// use actyxos_sdk::{fish_name, legacy::FishName};
/// let fish_name: FishName = fish_name!("abc");
/// ```
/// This does not compile:
/// ```compile_fail
/// use actyxos_sdk::{fish_name, legacy::FishName};
/// let fish_name: FishName = fish_name!("");
/// ```
#[macro_export]
macro_rules! fish_name {
    ($lit:tt) => {{
        #[allow(dead_code)]
        type X = $crate::assert_len!($lit, 1..);
        use ::std::convert::TryFrom;
        $crate::legacy::FishName::try_from($lit).unwrap()
    }};
}

/// Macro for constructing a [`SourceId`](event/struct.SourceId.html) literal.
///
/// This is how it works:
/// ```no_run
/// use actyxos_sdk::{source_id, legacy::SourceId};
/// let source_id: SourceId = source_id!("abc");
/// ```
/// This does not compile:
/// ```compile_fail
/// use actyxos_sdk::{source_id, legacy::SourceId};
/// let source_id: SourceId = source_id!("");
/// ```
#[macro_export]
macro_rules! source_id {
    ($lit:tt) => {{
        #[allow(dead_code)]
        type X = $crate::assert_len!($lit, 1..=15);
        use ::std::convert::TryFrom;
        $crate::legacy::SourceId::try_from($lit).unwrap()
    }};
}

// DO NOT FORGET TO UPDATE THE VALUE IN THE MACRO ABOVE!
pub(crate) const MAX_SOURCEID_LENGTH: usize = 15;

mk_scalar!(
    /// The semantics denotes a certain kind of fish and usually implies a certain type
    /// of payloads.
    ///
    /// For more on Fishes see the documentation on [Actyx Pond](https://developer.actyx.com/docs/pond/getting-started).
    /// You may most conveniently construct values of this type with the [`semantics!`](../macro.semantics.html) macro.
    struct Semantics, EmptySemantics
);

impl Semantics {
    /// Placeholder given to v1-style `Semantics`, when using the tagged API without providing
    /// an explicit `semantics` tag.
    pub fn unknown() -> Self {
        semantics!("_t_")
    }
}

mk_scalar!(
    /// The name identifies a particular instance of a Fish, i.e. one of a given kind as identified by
    /// its semantics.
    ///
    /// For more on Fishes see the documentation on [Actyx Pond](https://developer.actyx.com/docs/pond/getting-started).
    /// You may most conveniently construct values of this type with the [`fish_name!`](../macro.fish_name.html) macro.
    struct FishName, EmptyFishName
);

impl FishName {
    /// Placeholder given to v1-style `FishName`, when using the tagged API without providing
    /// an explicit `fish_name` tag.
    pub fn unknown() -> Self {
        fish_name!("_t_")
    }
}

impl TryFrom<&TagSet> for Semantics {
    type Error = anyhow::Error;
    fn try_from(value: &TagSet) -> Result<Self, Self::Error> {
        let sem = value
            .iter()
            .filter(|t| t.to_string().starts_with("semantics:"))
            .filter_map(|t| {
                let t = t.to_string();
                let pos = t.find(':')?;
                Semantics::try_from(&t[pos + 1..]).ok()
            })
            .collect::<Vec<_>>();
        if sem.len() == 1 {
            sem.into_iter().next().ok_or_else(|| anyhow!("cannot happen"))
        } else {
            Err(anyhow!("no unique semantics tag found"))
        }
    }
}

impl TryFrom<&TagSet> for FishName {
    type Error = anyhow::Error;
    fn try_from(value: &TagSet) -> Result<Self, Self::Error> {
        let names = value
            .iter()
            .filter(|t| t.to_string().starts_with("fish_name:"))
            .filter_map(|t| {
                let t = t.to_string();
                let pos = t.find(':')?;
                FishName::try_from(&t[pos + 1..]).ok()
            })
            .collect::<Vec<_>>();
        if names.len() == 1 {
            names.into_iter().next().ok_or_else(|| anyhow!("cannot happen"))
        } else {
            Err(anyhow!("no unique fish_name tag found"))
        }
    }
}

/// Hold provenance information for this event
///
/// Each event is published by one ActyxOS node whose source ID is stored in the `source` field.
/// [`Semantics`](struct.Semantics.html) & [`FishName`](struct.FishName.html) are metadata tags
/// that split the overall distributed event stream accessible by ActyxOS into smaller substreams
/// containing information about kinds of things (like sensor readings) and specific instances of
/// those things (like a thermometer’s name).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(rename_all = "camelCase")]
pub struct StreamInfo {
    pub semantics: Semantics,
    pub name: FishName,
    pub source: SourceId,
}

/// A source ID uniquely identifies one ActyxOS node
///
/// You can obtain the node’s source ID using [`EventService::node_id`](../event_service/struct.EventService.html#method.node_id).
/// It is mostly used in creating specific event stream queries involving
/// [`Subscription::local`](../event_service/struct.Subscription.html#method.local).
// SourceId is ordered by unicode code-point, which with UTF-8 is identical to
// byte-wise ordering. Since NUL is a valid code-point, the derived Ord and
// PartialOrd are only correct because a trailing NUL will result in the length
// byte being larger, hence leading to the correct ordering.
//
// TODO change to u128 to make it even more optimal
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct SourceId(pub(crate) [u8; MAX_SOURCEID_LENGTH + 1]);

impl SourceId {
    pub fn new(s: String) -> Result<Self, ParseError> {
        Self::from_str(s.as_ref())
    }

    pub fn as_str(&self) -> &str {
        let length = self.0[MAX_SOURCEID_LENGTH] as usize;
        std::str::from_utf8(&self.0[0..length]).expect("content must be valid utf8 string")
    }

    pub fn is_wildcard(&self) -> bool {
        self.is_empty()
    }

    /// true if the string representation of the source id is the empty string
    pub fn is_empty(&self) -> bool {
        self.0[MAX_SOURCEID_LENGTH] as usize == 0
    }
}

impl Debug for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "SourceId({})", self.as_str())
    }
}

impl TryFrom<&str> for SourceId {
    type Error = ParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl FromStr for SourceId {
    type Err = ParseError;

    fn from_str(text: &str) -> Result<SourceId, ParseError> {
        let bytes = text.as_bytes();
        if bytes.len() > MAX_SOURCEID_LENGTH {
            return Result::Err(ParseError::SourceIdTooLong);
        }
        if bytes.is_empty() {
            return Result::Err(ParseError::EmptySourceId);
        }
        let mut buf = [0; MAX_SOURCEID_LENGTH + 1];
        buf[MAX_SOURCEID_LENGTH] = bytes.len() as u8;
        buf[..bytes.len()].clone_from_slice(bytes);
        Result::Ok(SourceId(buf))
    }
}

// impl Into<StreamId> for SourceId {
//     unimplemented!()
// }

// impl Into<StreamId> for SourceId {
//     fn into(src: SourceId) -> Self {
//         let mut bytes = [0u8; 32];
//         bytes[0..=MAX_SOURCEID_LENGTH].copy_from_slice(&src.0[..]);
//         StreamId {
//             node_id: NodeId(bytes),
//             stream_nr: 0.into(),
//         }
//     }
// }

impl Display for SourceId {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.as_str())
    }
}

impl<'de> Deserialize<'de> for SourceId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<SourceId, D::Error> {
        nonempty_string(deserializer).and_then(|arc| SourceId::try_from(&*arc).map_err(D::Error::custom))
    }
}

impl Serialize for SourceId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        assert_eq!(
            serde_json::from_str::<Semantics>(r#""abc""#).unwrap(),
            semantics!("abc")
        );
        let res = serde_json::from_str::<Semantics>("\"\"").unwrap_err();
        assert_eq!(res.to_string(), "expected non-empty string");
    }

    #[test]
    fn deserialize_owned() {
        assert_eq!(
            serde_json::from_reader::<_, Semantics>(br#""abc""#.as_ref()).unwrap(),
            semantics!("abc")
        );
        let res = serde_json::from_reader::<_, Semantics>(b"\"\"".as_ref()).unwrap_err();
        assert_eq!(res.to_string(), "expected non-empty string");
        assert_eq!(
            serde_json::from_reader::<_, SourceId>(br#""abc""#.as_ref()).unwrap(),
            source_id!("abc")
        );
        let res = serde_json::from_reader::<_, SourceId>(b"\"\"".as_ref()).unwrap_err();
        assert_eq!(res.to_string(), "expected non-empty string");
    }

    #[test]
    fn reject_empty_source_id() {
        SourceId::from_str("").unwrap_err();
    }
}
