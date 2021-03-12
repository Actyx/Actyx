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
use std::{cmp::Ordering, convert::TryFrom};

use serde::{Deserialize, Serialize};

use crate::{scalars::StreamId, tags::TagSet, LamportTimestamp, Offset, Payload, TimeStamp};

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
/// use actyxos_sdk::{tagged::Event, Payload};
///
/// #[derive(Serialize, Deserialize, Debug, Clone)]
/// struct MyPayload {
///     x: f64,
///     y: Option<f64>,
/// }
///
/// let payload = Payload::from_json_str(r#"{"x":1.3}"#).unwrap();
/// let event: Event<Payload> = Event::from_payload(payload);
/// let my_event: Event<MyPayload> = event.extract::<MyPayload>().expect("expected MyPayload");
/// ```
#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(rename_all = "camelCase")]
pub struct Event<T> {
    pub key: EventKey,
    pub meta: Metadata,
    pub payload: T,
}

impl<T> PartialOrd for Event<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.key.cmp(&other.key))
    }
}

impl<T> Ord for Event<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

impl<T> PartialEq for Event<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<T> Eq for Event<T> {}

impl<T> Event<T> {
    /// Construct a default event with fake event key and metadata containing the given payload
    pub fn from_payload(payload: T) -> Self {
        Event::<Payload>::default().with_payload(payload)
    }

    /// Replace the payload in this event with the given one, keeping the event key and metadata
    pub fn with_payload<U>(self, u: U) -> Event<U> {
        Event {
            key: self.key,
            meta: self.meta,
            payload: u,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub timestamp: TimeStamp,
    pub tags: TagSet,
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
            key: self.key,
            meta: self.meta.clone(),
            payload,
        })
    }
}

/// The sort key of an event
///
/// It is sorted first by Lamport timestamp and then by source ID; this combination is already
/// unique. The offset is included to keep track of progress in [`OffsetMap`](struct.OffsetMap.html).
#[derive(Copy, Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(rename_all = "camelCase")]
pub struct EventKey {
    pub lamport: LamportTimestamp,
    pub stream: StreamId,
    pub offset: Offset,
}

/// The default value is the smallest key according to the EventKey order, and it should not be
/// equal to any event key generated by a live system (because the first generated Lamport timestamp
/// will be 1 while the default is 0).
impl Default for EventKey {
    fn default() -> Self {
        Self {
            lamport: Default::default(),
            stream: StreamId::try_from("!").unwrap(), // FIXME
            offset: Default::default(),
        }
    }
}
