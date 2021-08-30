use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::{
    offset::Offset,
    scalars::StreamId,
    tags::TagSet,
    timestamp::{LamportTimestamp, Timestamp},
    AppId,
};

mod opaque;
mod payload;

pub use opaque::Opaque;
pub use payload::Payload;

/// Events are delivered in this envelope together with their metadata
///
/// # Metadata
///
/// Ordering and equality do not depend on the type of payload: `lamport` and `key.stream`
/// uniquely identify the event and give rise to a total order (first by Lamport timestamp,
/// then by stream ID;  an Actyx node   will never use the same Lamport timestamp
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
/// The envelope contains a generic `Payload` payload type, you may use
/// the [`extract`](#method.extract) method to parse the payload as a more specific object.
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use actyx_sdk::{Event, Payload};
///
/// #[derive(Serialize, Deserialize, Debug, Clone)]
/// struct MyPayload {
///     x: f64,
///     y: Option<f64>,
/// }
///
/// let payload = Payload::from_json_str(r#"{"x":1.3}"#).unwrap();
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(rename_all = "camelCase")]
pub struct Event<T> {
    /// Uniquely identifying event key. Used for sorting.
    pub key: EventKey,
    /// Metadata incl. tags, timestamp and app ID
    pub meta: Metadata,
    /// The actual event payload
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

/// Metadata attached to an event that can be used for filtering.
#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    /// Timestamp of when the event was created
    pub timestamp: Timestamp,
    /// Attached tags
    pub tags: TagSet,
    /// ID of the app that emitted this event
    pub app_id: AppId,
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
/// It is sorted first by Lamport timestamp and then by stream ID; this combination is already
/// unique. The offset is included to keep track of progress in [`OffsetMap`](struct.OffsetMap.html).
#[derive(Copy, Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(rename_all = "camelCase")]
pub struct EventKey {
    pub lamport: LamportTimestamp,
    pub stream: StreamId,
    pub offset: Offset,
}
