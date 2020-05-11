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
use derive_more::Display;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    convert::TryFrom,
    fmt::{self, Debug, Display, Formatter},
    ops::{Add, Deref, Sub},
    str::FromStr,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

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
}
impl std::error::Error for ParseError {}

fn nonempty_string<'de, D: Deserializer<'de>>(d: D) -> Result<ArcVal<str>, D::Error> {
    let s = <&str>::deserialize(d)?;
    if s.is_empty() {
        Err(D::Error::custom("expected non-empty string"))
    } else {
        Ok(s.into())
    }
}

/// Macro for constructing a [`Semantics`](event/struct.Semantics.html) literal.
///
/// This is how it works:
/// ```no_run
/// use actyxos_sdk::{semantics, event::Semantics};
/// let semantics: Semantics = semantics!("abc");
/// ```
/// This does not compile:
/// ```compile_fail
/// use actyxos_sdk::{semantics, event::Semantics};
/// let semantics: Semantics = semantics!("");
/// ```
#[macro_export]
macro_rules! semantics {
    ($lit:tt) => {{
        #[allow(dead_code)]
        type X = $crate::assert_len!($lit, 1..);
        use ::std::convert::TryFrom;
        $crate::event::Semantics::try_from($lit).unwrap()
    }};
}

/// Macro for constructing a [`FishName`](event/struct.FishName.html) literal.
///
/// This is how it works:
/// ```no_run
/// use actyxos_sdk::{fish_name, event::FishName};
/// let fish_name: FishName = fish_name!("abc");
/// ```
/// This does not compile:
/// ```compile_fail
/// use actyxos_sdk::{fish_name, event::FishName};
/// let fish_name: FishName = fish_name!("");
/// ```
#[macro_export]
macro_rules! fish_name {
    ($lit:tt) => {{
        #[allow(dead_code)]
        type X = $crate::assert_len!($lit, 1..);
        use ::std::convert::TryFrom;
        $crate::event::FishName::try_from($lit).unwrap()
    }};
}

/// Macro for constructing a [`Tag`](event/struct.Tag.html) literal.
///
/// This is how it works:
/// ```no_run
/// use actyxos_sdk::{tag, event::Tag};
/// let tag: Tag = tag!("abc");
/// ```
/// This does not compile:
/// ```compile_fail
/// use actyxos_sdk::{tag, event::Tag};
/// let tag: Tag = tag!("");
/// ```
#[macro_export]
macro_rules! tag {
    ($lit:tt) => {{
        #[allow(dead_code)]
        type X = $crate::assert_len!($lit, 1..);
        use ::std::convert::TryFrom;
        $crate::event::Tag::try_from($lit).unwrap()
    }};
}

/// Macro for constructing a [`SourceId`](event/struct.SourceId.html) literal.
///
/// This is how it works:
/// ```no_run
/// use actyxos_sdk::{source_id, event::SourceId};
/// let source_id: SourceId = source_id!("abc");
/// ```
/// This does not compile:
/// ```compile_fail
/// use actyxos_sdk::{source_id, event::SourceId};
/// let source_id: SourceId = source_id!("");
/// ```
#[macro_export]
macro_rules! source_id {
    ($lit:tt) => {{
        #[allow(dead_code)]
        type X = $crate::assert_len!($lit, 1..=15);
        use ::std::convert::TryFrom;
        $crate::event::SourceId::try_from($lit).unwrap()
    }};
}

/// Macro for constructing a set of [`Tag`](event/struct.Tag.html) values.
///
/// The values accepted are either
///  - non-empty string literals
///  - normal expressions (enclosed in parens if multiple tokens)
///
/// ```rust
/// use actyxos_sdk::{tag, tags, semantics, event::{Semantics, Tag}};
/// use std::collections::BTreeSet;
///
/// let sem: Semantics = semantics!("b");
/// let tags: BTreeSet<Tag> = tags!("a", sem);
///
/// let mut expected = BTreeSet::new();
/// expected.insert(tag!("a"));
/// expected.insert(tag!("semantics:b"));
/// assert_eq!(tags, expected);
/// ```
#[macro_export]
macro_rules! tags {
    ($($expr:tt),*) => {{
        let mut _tags = ::std::collections::BTreeSet::new();
        $(
            {
                mod y {
                    $crate::assert_len! { $expr, 1..,
                        pub fn x(z: &str) -> $crate::event::Tag {
                            use ::std::convert::TryFrom;
                            $crate::event::Tag::try_from(z).unwrap()
                        },
                        pub fn x(z: impl Into<$crate::event::Tag>) -> $crate::event::Tag {
                            z.into()
                        }
                    }
                }
                #[allow(unused_parens)]
                _tags.insert(y::x($expr));
            }
        )*
        _tags
    }};
    ($($x:tt)*) => {
        compile_error!("This macro supports only string literals or expressions in parens.")
    }
}

// DO NOT FORGET TO UPDATE THE VALUE IN THE MACRO ABOVE!
const MAX_SOURCEID_LENGTH: usize = 15;

macro_rules! mk_scalar {
    ($(#[$attr:meta])* struct $id:ident, $err:ident) => {

        $(#[$attr])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "dataflow", derive(Abomonation))]
        pub struct $id(
            #[serde(deserialize_with = "nonempty_string")]
            ArcVal<str>
        );

        impl $id {
            pub fn new(value: String) -> Result<Self, ParseError> {
                if value.is_empty() {
                    Err(ParseError::$err)
                } else {
                    Ok(Self(value.as_str().into()))
                }
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
            pub fn as_arc(&self) -> &Arc<str> {
                &self.0.as_arc()
            }
        }

        impl TryFrom<&str> for $id {
            type Error = ParseError;
            fn try_from(value: &str) -> Result<Self, ParseError> {
                if value.is_empty() {
                    Err(ParseError::$err)
                } else {
                    Ok(Self(value.into()))
                }
            }
        }

        impl TryFrom<Arc<str>> for $id {
            type Error = ParseError;
            fn try_from(value: Arc<str>) -> Result<Self, ParseError> {
                if value.is_empty() {
                    Err(ParseError::$err)
                } else {
                    Ok(Self(value.into()))
                }
            }
        }

        impl Deref for $id {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                self.0.as_ref()
            }
        }
    };
}

mk_scalar!(
    /// The semantics denotes a certain kind of fish and usually implies a certain type
    /// of payloads.
    ///
    /// For more on Fishes see the documentation on [Actyx Pond](https://developer.actyx.com/docs/pond/getting-started).
    /// You may most conveniently construct values of this type with the [`semantics!`](../macro.semantics.html) macro.
    struct Semantics, EmptySemantics
);

mk_scalar!(
    /// The name identifies a particular instance of a Fish, i.e. one of a given kind as identified by
    /// its semantics.
    ///
    /// For more on Fishes see the documentation on [Actyx Pond](https://developer.actyx.com/docs/pond/getting-started).
    /// You may most conveniently construct values of this type with the [`fish_name!`](../macro.fish_name.html) macro.
    struct FishName, EmptyFishName
);

mk_scalar!(
    /// Arbitrary metadata-string for an Event. Website documentation pending.
    ///
    /// You may most conveniently construct values of this type with the [`tag!`](../macro.tag.html) macro.
    struct Tag, EmptyTag
);

impl From<&Semantics> for Tag {
    fn from(value: &Semantics) -> Self {
        Tag::new(format!("semantics:{}", value.as_str())).unwrap()
    }
}

impl From<Semantics> for Tag {
    fn from(value: Semantics) -> Self {
        Tag::from(&value)
    }
}

impl From<&FishName> for Tag {
    fn from(value: &FishName) -> Self {
        Tag::new(format!("fish_name:{}", value.as_str())).unwrap()
    }
}

impl From<FishName> for Tag {
    fn from(value: FishName) -> Self {
        Tag::from(&value)
    }
}

/// Microseconds since the UNIX epoch, without leap seconds and in UTC
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct TimeStamp(u64);

impl TimeStamp {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
    pub fn now() -> TimeStamp {
        let now = SystemTime::now();
        let duration = now
            .duration_since(UNIX_EPOCH)
            .expect("Time went waaaay backwards");
        TimeStamp::new(duration.as_micros() as u64)
    }
    pub fn as_u64(self) -> u64 {
        self.0
    }
    pub fn as_i64(self) -> i64 {
        self.0 as i64
    }
}

impl Sub<u64> for TimeStamp {
    type Output = TimeStamp;
    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Sub<TimeStamp> for TimeStamp {
    type Output = i64;
    fn sub(self, rhs: TimeStamp) -> Self::Output {
        self.0 as i64 - rhs.0 as i64
    }
}

impl Add<u64> for TimeStamp {
    type Output = TimeStamp;
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

/// A logical timestamp taken from a [`Lamport clock`](https://en.wikipedia.org/wiki/Lamport_timestamps)
///
/// The lamport clock in an ActyxOS system is increased by the ActyxOS node whenever:
///
/// - an event is emitted
/// - a heartbeat is received
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct LamportTimestamp(u64);

impl LamportTimestamp {
    pub fn new(value: u64) -> Self {
        Self(value)
    }
    pub fn incr(self) -> Self {
        LamportTimestamp::new(self.0 + 1)
    }
    pub fn as_u64(self) -> u64 {
        self.0
    }
    pub fn as_i64(self) -> i64 {
        self.0 as i64
    }
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
pub struct SourceId([u8; MAX_SOURCEID_LENGTH + 1]);

impl SourceId {
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
        let mut buf = [0; MAX_SOURCEID_LENGTH + 1];
        buf[MAX_SOURCEID_LENGTH] = bytes.len() as u8;
        buf[..bytes.len()].clone_from_slice(&bytes[..]);
        Result::Ok(SourceId(buf))
    }
}

impl Display for SourceId {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.as_str())
    }
}

impl<'de> Deserialize<'de> for SourceId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<SourceId, D::Error> {
        struct SourceIdVisitor;

        impl<'de> Visitor<'de> for SourceIdVisitor {
            type Value = SourceId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("A valid SourceId")
            }

            // JSON variant
            fn visit_str<E>(self, string: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Result::Ok(SourceId::from_str(string).map_err(serde::de::Error::custom)?)
            }

            fn visit_string<E>(self, string: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Result::Ok(SourceId::from_str(&*string).map_err(serde::de::Error::custom)?)
            }
        }
        deserializer.deserialize_str(SourceIdVisitor)
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
    use std::collections::BTreeSet;

    #[test]
    fn semantics_to_tag() {
        let semantics = semantics!("test");

        assert_eq!("semantics:test", Tag::from(&semantics).as_str());
        assert_eq!("semantics:test", Tag::from(semantics).as_str());
    }

    #[test]
    fn fish_name_to_tag() {
        let fish_name = fish_name!("test");

        assert_eq!("fish_name:test", Tag::from(&fish_name).as_str());
        assert_eq!("fish_name:test", Tag::from(fish_name).as_str());
    }

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
    fn make_tags() {
        let sem_b = semantics!("b");
        let mut tags = BTreeSet::new();
        tags.insert(tag!("a"));
        tags.insert(tag!("semantics:b"));
        // parens are there on purpose to make sure that clippy is silenced
        assert_eq!(tags!("a", (sem_b)), tags);
    }
}
