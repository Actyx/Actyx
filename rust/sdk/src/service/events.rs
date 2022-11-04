use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Display, num::NonZeroU64, ops::AddAssign};

use crate::{
    app_id,
    event::{Event, EventKey, Metadata},
    scalars::StreamId,
    tags::TagSet,
    LamportTimestamp, Offset, OffsetMap, Payload, Timestamp,
};
use lazy_static::lazy_static;

/// The order in which you want to receive events for a query
///
/// Event streams can be requested with different ordering requirements from the
/// Event Service:
///
///  - in strict ascending order
///  - in strict descending order
///  - ordered in ascending order per stream, but not across streams
#[derive(Debug, Serialize, Deserialize, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Order {
    /// Events are sorted by ascending Lamport timestamp and stream ID, which defines a
    /// total order.
    Asc,
    /// Events are sorted by descending Lamport timestamp and descending stream ID,
    /// which is the exact reverse of the `Asc` ordering.
    Desc,
    /// Events are sorted within each stream by ascending Lamport timestamp, with events
    /// from different streams interleaved in an undefined order.
    StreamAsc,
}

/// Query for a bounded set of events across multiple event streams.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QueryRequest {
    /// Optional lower bound offset per stream.
    pub lower_bound: Option<OffsetMap>,
    /// Upper bound offset per stream.
    pub upper_bound: Option<OffsetMap>,
    /// Query for which events should be returned.
    pub query: String,
    /// Order in which events should be received.
    pub order: Order,
}

/// Subscription to an unbounded set of events across multiple streams.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeRequest {
    /// Optional lower bound offset per stream.
    pub lower_bound: Option<OffsetMap>,
    /// Query for which events should be returned.
    pub query: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(from = "EventMetaIo", into = "EventMetaIo")]
pub enum EventMeta {
    Range {
        from_key: EventKey,
        to_key: EventKey,
        from_time: Timestamp,
        to_time: Timestamp,
    },
    Synthetic,
    Event {
        key: EventKey,
        meta: Metadata,
    },
}
#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(untagged)]
pub enum EventMetaIo {
    #[serde(rename_all = "camelCase")]
    Range {
        from_key: EventKey,
        to_key: EventKey,
        from_time: Timestamp,
        to_time: Timestamp,
        #[serde(flatten)]
        key: EventKey,
        #[serde(flatten)]
        meta: Metadata,
    },
    Event {
        #[serde(flatten)]
        key: EventKey,
        #[serde(flatten)]
        meta: Metadata,
    },
}
lazy_static! {
    static ref METADATA: Metadata = Metadata {
        timestamp: Timestamp::new(0),
        tags: TagSet::empty(),
        app_id: app_id!("none"),
    };
}
impl From<EventMeta> for EventMetaIo {
    fn from(em: EventMeta) -> Self {
        match em {
            EventMeta::Range {
                from_key,
                to_key,
                from_time,
                to_time,
            } => Self::Range {
                from_key,
                to_key,
                from_time,
                to_time,
                key: EventKey::ZERO,
                meta: METADATA.clone(),
            },
            EventMeta::Synthetic => Self::Event {
                key: EventKey::ZERO,
                meta: METADATA.clone(),
            },
            EventMeta::Event { key, meta } => Self::Event { key, meta },
        }
    }
}
impl From<EventMetaIo> for EventMeta {
    fn from(em: EventMetaIo) -> Self {
        match em {
            EventMetaIo::Range {
                from_key,
                to_key,
                from_time,
                to_time,
                ..
            } => Self::Range {
                from_key,
                to_key,
                from_time,
                to_time,
            },
            EventMetaIo::Event { key, meta } => {
                if meta.timestamp.as_i64() == 0 {
                    Self::Synthetic
                } else {
                    Self::Event { key, meta }
                }
            }
        }
    }
}
impl EventMeta {
    fn left(&self) -> (EventKey, Timestamp) {
        match self {
            EventMeta::Range {
                from_key, from_time, ..
            } => (*from_key, *from_time),
            EventMeta::Synthetic => (EventKey::ZERO, 0.into()),
            EventMeta::Event { key, meta } => (*key, meta.timestamp),
        }
    }
    fn right(&self) -> (EventKey, Timestamp) {
        match self {
            EventMeta::Range { to_key, to_time, .. } => (*to_key, *to_time),
            EventMeta::Synthetic => (EventKey::ZERO, 0.into()),
            EventMeta::Event { key, meta } => (*key, meta.timestamp),
        }
    }
}
impl AddAssign<&Self> for EventMeta {
    fn add_assign(&mut self, rhs: &Self) {
        if *rhs == EventMeta::Synthetic {
            return;
        }
        match self {
            EventMeta::Range {
                from_key,
                to_key,
                from_time,
                to_time,
            } => {
                // this only works because we excluded rhs == Synthetic above
                let (min_key, min_time) = rhs.left();
                let (max_key, max_time) = rhs.right();
                if min_key < *from_key {
                    *from_key = min_key;
                }
                if max_key > *to_key {
                    *to_key = max_key;
                }
                if min_time < *from_time {
                    *from_time = min_time;
                }
                if max_time > *to_time {
                    *to_time = max_time;
                }
            }
            EventMeta::Synthetic => *self = rhs.clone(),
            EventMeta::Event { key, meta } => match rhs {
                EventMeta::Range {
                    from_key: min_key,
                    to_key: max_key,
                    from_time: min_time,
                    to_time: max_time,
                } => {
                    *self = EventMeta::Range {
                        from_key: (*key).min(*min_key),
                        to_key: (*key).max(*max_key),
                        from_time: (meta.timestamp).min(*min_time),
                        to_time: (meta.timestamp).max(*max_time),
                    };
                }
                EventMeta::Synthetic => {}
                EventMeta::Event {
                    key: rkey,
                    meta: Metadata { timestamp: rtime, .. },
                } => {
                    if rkey == key && *rtime == meta.timestamp {
                        return;
                    }
                    *self = EventMeta::Range {
                        from_key: (*rkey).min(*key),
                        to_key: (*rkey).max(*key),
                        from_time: (*rtime).min(meta.timestamp),
                        to_time: (*rtime).max(meta.timestamp),
                    };
                }
            },
        }
    }
}

/// Event response
#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventResponse<T> {
    #[serde(flatten)]
    pub meta: EventMeta,
    /// The actual, app-specific event payload
    pub payload: T,
}
impl<T> From<Event<T>> for EventResponse<T> {
    fn from(env: Event<T>) -> Self {
        let Event { key, meta, payload } = env;
        EventResponse {
            meta: EventMeta::Event { key, meta },
            payload,
        }
    }
}

impl EventResponse<Payload> {
    /// Try to extract the given type from the generic payload and return a new
    /// event envelope if successful. The produced payload is deserialized as efficiently
    /// as possible and may therefore still reference memory owned by the `Payload`.
    /// You may need to `.clone()` it to remove this dependency.
    pub fn extract<'a, T>(&'a self) -> Result<EventResponse<T>, serde_cbor::Error>
    where
        T: Deserialize<'a> + Clone,
    {
        Ok(EventResponse {
            meta: self.meta.clone(),
            payload: self.payload.extract::<T>()?,
        })
    }
}

impl<T> std::fmt::Display for EventResponse<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use chrono::TimeZone;
        match &self.meta {
            EventMeta::Range { .. } => {
                write!(f, "composite event")
            }
            EventMeta::Event { key, meta } => {
                let time = chrono::Local.timestamp_millis(meta.timestamp.as_i64() / 1000);
                write!(
                    f,
                    "event at {} ({}, stream ID {})",
                    time.to_rfc3339_opts(chrono::SecondsFormat::Millis, false),
                    key.lamport,
                    key.stream,
                )
            }
            EventMeta::Synthetic => f.write_str("synthetic event"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OffsetMapResponse {
    pub offsets: OffsetMap,
}

/// Publication of an event
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublishEvent {
    /// Attached tags
    pub tags: TagSet,
    /// App-specific event payload
    pub payload: Payload,
}

/// Publication of a set of events
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublishRequest {
    /// Events to be published
    pub data: Vec<PublishEvent>,
}

/// Result of an event publication
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublishResponseKey {
    /// Lamport timestamp
    pub lamport: LamportTimestamp,
    /// Associated stream's ID
    pub stream: StreamId,
    /// Offset within the associated stream
    pub offset: Offset,
    /// Timestamp at which the event was stored by the service
    pub timestamp: Timestamp,
}

/// Result of the publication of a set of events
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublishResponse {
    /// Metadata for each published event
    pub data: Vec<PublishResponseKey>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum StartFrom {
    /// If the lower bound is given in, it filters out all events that are included
    /// in the offset map.
    LowerBound(OffsetMap),
}

/// The session identifier used in /subscribe_monotonic
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SessionId(Box<str>);

impl Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl SessionId {
    /// Extracts a string slice containing the entire session id
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Subscribe to live updates as the Event Services receives or publishes new events,
/// until the recipient would need to time travel
///
/// Time travel is defined as receiving an event that needs to be sorted earlier than
/// an event that has already been received.
///
/// Send this request to retrieve an unbounded stream of events.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeMonotonicRequest {
    /// This id uniquely identifies one particular session. Connecting again with this
    /// SessionId shall only be done after a TimeTravel message has been received. The
    /// subscription is stored with the Session and all previous state is destroyed
    /// upon receiving a different subscription for this session.
    pub session: SessionId,
    /// Definition of the events to be received by this session, i.e. a selection of
    /// tags coupled with other flags like “isLocal”.
    pub query: String,
    /// The consumer may already have kept state and know at which point to resume a
    /// previously interrupted stream. In this case, StartFrom::Offsets is used,
    /// otherwise StartFrom::Snapshot indicates that the PondService shall figure
    /// out where best to start out from, possibly sending a `State` message first.
    #[serde(flatten)]
    pub from: StartFrom,
}

/// The response to a monotonic subscription is a stream of events terminated by a time travel.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SubscribeMonotonicResponse {
    /// This is the main message, a new event that is to be applied directly to the
    /// currently known state to produce the next state.
    #[serde(rename_all = "camelCase")]
    Event {
        #[serde(flatten)]
        event: EventResponse<Payload>,
        caught_up: bool,
    },
    #[serde(rename_all = "camelCase")]
    Offsets(OffsetMapResponse),
    /// This message ends the stream in case a replay becomes necessary due to
    /// time travel. The contained event key signals how far back the replay will
    /// reach so that the consumer can invalidate locally stored snapshots (if
    /// relevant).
    #[serde(rename_all = "camelCase")]
    TimeTravel { new_start: EventKey },
    #[serde(rename_all = "camelCase")]
    Diagnostic(Diagnostic),
    #[serde(other)]
    FutureCompat,
}

/// The response to a query request.
///
/// This will currently only be elements of type `Event` but will eventually contain
/// `Offset`s to communicate progress of events not included in the query.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum QueryResponse {
    #[serde(rename_all = "camelCase")]
    Event(EventResponse<Payload>),
    #[serde(rename_all = "camelCase")]
    Offsets(OffsetMapResponse),
    #[serde(rename_all = "camelCase")]
    Diagnostic(Diagnostic),
    #[serde(other)]
    FutureCompat,
}

/// The response to a subscribe request.
///
/// This will currently only be elements of type `Event` but will eventually contain
/// `Offset`s to communicate progress of events not included in the query.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SubscribeResponse {
    #[serde(rename_all = "camelCase")]
    Event(EventResponse<Payload>),
    #[serde(rename_all = "camelCase")]
    Offsets(OffsetMapResponse),
    #[serde(rename_all = "camelCase")]
    Diagnostic(Diagnostic),
    #[serde(other)]
    FutureCompat,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
}

impl Diagnostic {
    pub fn warn(message: String) -> Self {
        Self {
            severity: Severity::Warning,
            message,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            severity: Severity::Error,
            message,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Warning,
    Error,
    #[serde(other)]
    FutureCompat,
}

/// Response to the offsets request
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OffsetsResponse {
    /// Currently validated [`OffsetMap`] locally available
    pub present: OffsetMap,
    /// Number of events per [`StreamId`] pending replication to this node
    pub to_replicate: BTreeMap<StreamId, NonZeroU64>,
}

#[async_trait]
/// A service providing retrieval of historic and live events, publishing of new events
/// and access to information about current stream offsets.
pub trait EventService: Clone + Send {
    /// Returns known offsets across local and replicated streams.
    async fn offsets(&self) -> Result<OffsetsResponse>;

    /// Publishes a set of new events.
    async fn publish(&self, request: PublishRequest) -> Result<PublishResponse>;

    /// Query events known at the time the request was reveived by the service.
    async fn query(&self, request: QueryRequest) -> Result<BoxStream<'static, QueryResponse>>;

    /// Suscribe to events that are currently known by the service followed by new "live" events.
    async fn subscribe(&self, request: SubscribeRequest) -> Result<BoxStream<'static, SubscribeResponse>>;

    /// Subscribe to events that are currently known by the service followed by new "live" events until
    /// the service learns about events that need to be sorted earlier than an event already received.
    async fn subscribe_monotonic(
        &self,
        request: SubscribeMonotonicRequest,
    ) -> Result<BoxStream<'static, SubscribeMonotonicResponse>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app_id, tags, AppId, NodeId};
    use quickcheck::quickcheck;

    #[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct EventResponseV1<T> {
        /// Lamport timestamp
        lamport: LamportTimestamp,
        /// ID of the stream this event belongs to
        stream: StreamId,
        /// The event offset within the stream
        offset: Offset,
        /// Timestamp at which the event was emitted
        timestamp: Timestamp,
        /// Tag attached to the event
        tags: TagSet,
        /// Associated app ID
        app_id: AppId,
        /// The actual, app-specific event payload
        payload: T,
    }

    #[test]
    fn future_compat() {
        assert_eq!(
            serde_json::from_str::<QueryResponse>(r#"{"type":"fromTheFuture","x":42}"#).unwrap(),
            QueryResponse::FutureCompat
        );
        assert_eq!(
            serde_json::from_str::<SubscribeResponse>(r#"{"type":"fromTheFuture","x":42}"#).unwrap(),
            SubscribeResponse::FutureCompat
        );
        assert_eq!(
            serde_json::from_str::<SubscribeMonotonicResponse>(r#"{"type":"fromTheFuture","x":42}"#).unwrap(),
            SubscribeMonotonicResponse::FutureCompat
        );
    }

    #[test]
    fn event_response_compat() {
        let stream = NodeId::from_bytes(b"abcdefghijklmnopqrstuvwxyz123456")
            .unwrap()
            .stream(12.into());
        let lamport = LamportTimestamp::from(42);
        let offset = Offset::from(43);
        let timestamp = Timestamp::from(44);
        let tags = tags!("a1", "b2");
        let app_id = app_id!("tester");
        let payload = Payload::from_json_str("100").unwrap();

        let old = serde_json::to_string(&EventResponseV1 {
            lamport,
            stream,
            offset,
            timestamp,
            tags: tags.clone(),
            app_id: app_id.clone(),
            payload: payload.clone(),
        })
        .unwrap();
        assert_eq!(
            serde_json::from_str::<EventResponse<Payload>>(&old).unwrap(),
            EventResponse {
                meta: EventMeta::Event {
                    key: EventKey {
                        lamport,
                        stream,
                        offset,
                    },
                    meta: Metadata {
                        timestamp,
                        tags: tags.clone(),
                        app_id: app_id.clone(),
                    }
                },
                payload: payload.clone(),
            }
        );

        let old_synthetic = serde_json::to_string(&EventResponseV1 {
            lamport,
            stream,
            offset,
            timestamp: 0.into(),
            tags: tags.clone(),
            app_id: app_id.clone(),
            payload: payload.clone(),
        })
        .unwrap();
        assert_eq!(
            serde_json::from_str::<EventResponse<Payload>>(&old_synthetic).unwrap(),
            EventResponse {
                meta: EventMeta::Synthetic,
                payload: payload.clone(),
            }
        );

        let new_synthetic = serde_json::to_string(&EventResponse {
            meta: EventMeta::Synthetic,
            payload: payload.clone(),
        })
        .unwrap();
        assert_eq!(
            serde_json::from_str::<EventResponseV1<Payload>>(&new_synthetic).unwrap(),
            EventResponseV1 {
                lamport: 0.into(),
                stream: NodeId::default().stream(0.into()),
                offset: 0.into(),
                timestamp: 0.into(),
                tags: tags!(),
                app_id: app_id!("none"),
                payload: payload.clone(),
            }
        );

        let new_event = serde_json::to_string(&EventResponse {
            meta: EventMeta::Event {
                key: EventKey {
                    lamport,
                    stream,
                    offset,
                },
                meta: Metadata {
                    timestamp,
                    tags: tags.clone(),
                    app_id: app_id.clone(),
                },
            },
            payload: payload.clone(),
        })
        .unwrap();
        assert_eq!(
            serde_json::from_str::<EventResponseV1<Payload>>(&new_event).unwrap(),
            EventResponseV1 {
                lamport,
                stream,
                offset,
                timestamp,
                tags,
                app_id,
                payload: payload.clone(),
            }
        );

        let new_range = serde_json::to_string(&EventResponse {
            meta: EventMeta::Range {
                from_key: EventKey {
                    lamport,
                    stream,
                    offset,
                },
                to_key: EventKey {
                    lamport,
                    stream,
                    offset,
                },
                from_time: timestamp,
                to_time: timestamp,
            },
            payload: payload.clone(),
        })
        .unwrap();
        assert_eq!(
            serde_json::from_str::<EventResponseV1<Payload>>(&new_range).unwrap(),
            EventResponseV1 {
                lamport: 0.into(),
                stream: NodeId::default().stream(0.into()),
                offset: 0.into(),
                timestamp: 0.into(),
                tags: tags!(),
                app_id: app_id!("none"),
                payload,
            }
        );
    }

    quickcheck! {
        fn event_meta_merge(m: Vec<EventMeta>) -> bool {
            let mut em = EventMeta::Synthetic;
            let mut min_key = None;
            let mut max_key = None;
            let mut min_time = None;
            let mut max_time = None;
            for m in m {
                if m != EventMeta::Synthetic {
                    let min = m.left();
                    let max = m.right();
                    min_key = min_key.map(|k: EventKey| k.min(min.0)).or(Some(min.0));
                    max_key = max_key.map(|k: EventKey| k.max(max.0)).or(Some(max.0));
                    min_time = min_time.map(|k: Timestamp| k.min(min.1)).or(Some(min.1));
                    max_time = max_time.map(|k: Timestamp| k.max(max.1)).or(Some(max.1));
                }
                em += &m;
            }
            let (from_key, from_time) = em.left();
            let (to_key, to_time) = em.right();
            em == EventMeta::Synthetic && min_key.is_none() ||
            min_key == max_key && min_time == max_time
                && matches!(em, EventMeta::Event { key, meta: Metadata { timestamp, .. }}
                    if key == min_key.unwrap() && timestamp == min_time.unwrap()) ||
            em == EventMeta::Range { from_key, to_key, from_time, to_time }
        }
    }
}
