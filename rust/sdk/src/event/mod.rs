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
//! Definition of the event envelope structure provided by ActyxOS
//!
//! When [publishing events](../event_service/struct.EventService.html#method.publish)
//! you provide only the payload with the [`Semantics`](struct.Semantics.html) &
//! [`FishName`](struct.FishName.html) meta-data. The Event Service will add administrative
//! information on where the event came from and when it was published.
//!
//! Querying events delivers the full set of information within the [`Event`](struct.Event.html)
//! data structure.
//!
//! # Payload definition
//!
//! You are free to define your own payload data types as you see fit, provided that they
//! come with serialization and deserialization instances for [`serde`](https://docs.rs/serde)
//! (and [`Abomonation`](https://docs.rs/abomonation) if you want to feed the
//! payload into [`differential-dataflow`](https://docs.rs/differential-dataflow)).
//!
//! As JSON only supports `f64` floating point numbers, you may want to look into
//! [`FixNum`](../types/struct.FixNum.html) or [`decorum`](https://docs.rs/decorum) for
//! deserializing to a type that supports equality and total ordering.

use chrono::{Local, SecondsFormat, TimeZone};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;

mod offsets;
mod opaque;
mod scalars;

pub use offsets::{Offset, OffsetMap};
pub use opaque::Opaque;
pub use scalars::{
    FishName, LamportTimestamp, Semantics, SourceId, SourceIdReadError, Tag, TimeStamp,
};

/// Events are delivered in this envelope together with their metadata
///
/// # Metadata
///
/// Ordering and equality do not depend on the type of payload: `lamport` and `stream.source`
/// uniquely identify the event and give rise to a total order (first by Lamport timestamp,
/// then by source ID; a source — an ActyxOS node — will never use the same Lamport timestamp
/// more than once).
///
/// The contained Lamport timestamp tracks the causal order of events, which may not
/// always be linked to or consistent with real time, especially when events were produced by devices
/// that were not communicating for some time period. This implies that the wall clock
/// `timestamp` may go backwards when consuming an ordered event stream (this would also
/// be the case due to clock skew between devices).
///
/// > Illustrative example: two groups of nodes are separated from each other for some time
/// period, for example due to traveling through an area without network coverage. Their logical
/// clocks may advance at different speeds because the number of events created in each group
/// may be different. When sorting by Lamport timestamp, the events from the group with the lower
/// event rate will tend to be sorted earlier than the events from the other group, regardless
/// of the wall clock time at which they occurred.
///
/// It is desirable to sort by Lamport timestamps because they provide the correct (intuitive)
/// sort order when nodes are properly communicating, which is the common case. Using device
/// hardware clocks has proven to be quite unreliable because they may jump forward and backward
/// due to human or machine error.
///
/// # Payload
///
/// The EventService client will provide a generic `Payload` payload type, you may use
/// the [`extract`](#method.extract) method to parse the payload as a more specific object.
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use actyxos_sdk::event::{Event, Payload};
///
/// #[derive(Serialize, Deserialize, Debug, Clone)]
/// struct MyPayload {
///     x: f64,
///     y: Option<f64>,
/// }
///
/// let event: Event<Payload> = Event::mk_test("semantics", "name", "{\"x\":42}").unwrap();
/// let my_event: Event<MyPayload> = event.extract::<MyPayload>().expect("expected MyPayload");
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(rename_all = "camelCase")]
pub struct Event<T> {
    pub lamport: LamportTimestamp,
    pub stream: StreamInfo,
    pub timestamp: TimeStamp,
    pub offset: Offset,
    pub payload: T,
}

impl Event<Payload> {
    /// Try to extract the given type from the generic payload and return a new
    /// event envelope if successful. The produced payload is deserialized as efficiently
    /// as possible and may therefore still reference memory owned by the `Payload`.
    /// You may need to `.clone()` it to remove this dependency.
    pub fn extract<'a, T>(&'a self) -> Result<Event<T>, serde_cbor::Error>
    where
        T: Deserialize<'a> + Clone,
    {
        let payload = self.payload.extract::<T>()?;
        Ok(Event {
            lamport: self.lamport,
            stream: self.stream.clone(),
            timestamp: self.timestamp,
            offset: self.offset,
            payload,
        })
    }

    /// Create an Event instance for testing purposes.
    ///
    /// **Caveat emptor:** This will not generate proper
    /// timestamps and two such events will compare equal to each other (so cannot be
    /// put into collections without first making their Lamport timestamps unique).
    pub fn mk_test(
        semantics: &str,
        name: &str,
        payload: &str,
    ) -> Result<Event<Payload>, serde_json::Error> {
        Ok(Event {
            lamport: Default::default(),
            timestamp: Default::default(),
            offset: Offset(0),
            stream: StreamInfo {
                semantics: Semantics::from(semantics),
                name: FishName::from(name),
                source: SourceId::from_str("dummy").unwrap(),
            },
            payload: serde_json::from_str(payload)?,
        })
    }
}

impl<T> Ord for Event<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.lamport
            .cmp(&other.lamport)
            .then(self.stream.source.cmp(&other.stream.source))
    }
}

impl<T> PartialOrd for Event<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> PartialEq for Event<T> {
    fn eq(&self, other: &Self) -> bool {
        self.lamport == other.lamport && self.stream.source == other.stream.source
    }
}

impl<T> Eq for Event<T> {}

impl<T> Display for Event<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let time = Local.timestamp_millis(self.timestamp.as_i64() / 1000);
        write!(
            f,
            "Event at {} (lamport {}, source ID {})",
            time.to_rfc3339_opts(SecondsFormat::Millis, false),
            self.lamport.as_u64(),
            self.stream.source,
        )
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

/// Compact binary storage of events created when they are received from the Event Service
///
/// see [`Event::extract`](struct.Event.html#method.extract) for supported ways of using the
/// data
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
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
