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
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;
use std::sync::Arc;
use std::{
    ops::{Add, Deref, Sub},
    time::{SystemTime, UNIX_EPOCH},
};

macro_rules! mk_scalar {
    ($(#[$attr:meta])* struct $id:ident) => {

	$(#[$attr])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "dataflow", derive(Abomonation))]
        pub struct $id(ArcVal<str>);

        impl $id {
            pub fn new(value: String) -> Self {
                Self(value.as_str().into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
            pub fn as_arc(&self) -> &Arc<str> {
                &self.0.as_arc()
            }
        }

        impl From<&str> for $id {
            fn from(value: &str) -> Self {
                Self(value.into())
            }
        }

        impl From<Arc<str>> for $id {
            fn from(value: Arc<str>) -> Self {
                Self(value.into())
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
    /// of payloads. For more on Fishes see the documentation on [Actyx Pond](https://developer.actyx.com/docs/pond/getting-started)
    struct Semantics
);

mk_scalar!(
    /// The name identifies a particular instance of a Fish, i.e. one of a given kind as identified by
    /// its semantics. For more on Fishes see the documentation on [Actyx Pond](https://developer.actyx.com/docs/pond/getting-started)
    struct FishName
);

mk_scalar!(
    /// Arbitrary metadata-string for an Event. Website documentation pending.
    struct Tag
);

/// Shorthand for creating a set of tags from `&str`s.
#[macro_export]
macro_rules! tags {
    ($($x:expr),*) => ({
        let mut _temp = std::collections::BTreeSet::<actyxos_sdk::event::Tag>::new();

        $(_temp.insert(actyxos_sdk::event::Tag::from($x));)*

	_temp
    });
}

impl From<&Semantics> for Tag {
    fn from(value: &Semantics) -> Self {
        Tag::new(format!("semantics:{}", value.as_str()))
    }
}

impl From<Semantics> for Tag {
    fn from(value: Semantics) -> Self {
        Tag::from(&value)
    }
}

impl From<&FishName> for Tag {
    fn from(value: &FishName) -> Self {
        Tag::new(format!("fish_name:{}", value.as_str()))
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

const MAX_SOURCEID_LENGTH: usize = 15;

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

#[derive(Debug, Display, PartialEq)]
pub enum SourceIdReadError {
    #[display(fmt = "SourceId was longer than maximum")]
    IdTooLong,
}

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

impl std::error::Error for SourceIdReadError {}

impl Debug for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "SourceId({})", self.as_str())
    }
}

impl FromStr for SourceId {
    type Err = SourceIdReadError;

    fn from_str(text: &str) -> Result<SourceId, SourceIdReadError> {
        let bytes = text.as_bytes();
        if bytes.len() > MAX_SOURCEID_LENGTH {
            return Result::Err(SourceIdReadError::IdTooLong);
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

    #[test]
    fn semantics_to_tag() {
        let semantics = Semantics::from("test");

        assert_eq!("semantics:test", Tag::from(&semantics).as_str());
        assert_eq!("semantics:test", Tag::from(semantics).as_str());
    }

    #[test]
    fn fish_name_to_tag() {
        let fish_name = FishName::from("test");

        assert_eq!("fish_name:test", Tag::from(&fish_name).as_str());
        assert_eq!("fish_name:test", Tag::from(fish_name).as_str());
    }
}
