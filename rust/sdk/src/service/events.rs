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
use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Display, num::NonZeroU64};

use crate::{
    event::{Event, EventKey, Metadata},
    language::Query,
    scalars::StreamId,
    tags::TagSet,
    types::Binary,
    LamportTimestamp, Offset, OffsetMap, Payload, Timestamp,
};

/// The order in which you want to receive events for a query
///
/// Event streams can be request with different ordering requirements from the
/// Event Service:
///
///  - in strict Asc Lamport order
///  - in strict Desc Lamport order
///  - ordered in Asc order per stream, but not between streams
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QueryRequest {
    pub lower_bound: Option<OffsetMap>,
    pub upper_bound: OffsetMap,
    pub query: Query,
    pub order: Order,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeRequest {
    pub offsets: Option<OffsetMap>,
    pub query: Query,
}

#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventResponse<T> {
    pub lamport: LamportTimestamp,
    pub stream: StreamId,
    pub offset: Offset,
    pub timestamp: Timestamp,
    pub tags: TagSet,
    pub payload: T,
}
impl<T> From<Event<T>> for EventResponse<T> {
    fn from(env: Event<T>) -> Self {
        let EventKey {
            lamport,
            stream,
            offset,
        } = env.key;
        let Metadata { timestamp, tags, .. } = env.meta;
        let payload = env.payload;
        EventResponse {
            lamport,
            stream,
            offset,
            timestamp,
            tags,
            payload,
        }
    }
}

#[cfg(test)]
impl EventResponse<Payload> {
    /// Try to extract the given type from the generic payload and return a new
    /// event envelope if successful. The produced payload is deserialized as efficiently
    /// as possible and may therefore still reference memory owned by the `Payload`.
    /// You may need to `.clone()` it to remove this dependency.
    pub fn extract<'a, T>(&'a self) -> EventResponse<T>
    where
        T: Deserialize<'a> + Clone,
    {
        EventResponse {
            stream: self.stream,
            lamport: self.lamport,
            offset: self.offset,
            timestamp: self.timestamp,
            tags: self.tags.clone(),
            payload: self.payload.extract::<T>().unwrap(),
        }
    }
}

impl<T> std::fmt::Display for EventResponse<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use chrono::TimeZone;
        let time = chrono::Local.timestamp_millis(self.timestamp.as_i64() / 1000);
        write!(
            f,
            "Event at {} ({}, stream ID {})",
            time.to_rfc3339_opts(chrono::SecondsFormat::Millis, false),
            self.lamport,
            self.stream,
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishEvent {
    pub tags: TagSet,
    pub payload: Payload,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishRequest {
    pub data: Vec<PublishEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishResponseKey {
    pub lamport: LamportTimestamp,
    pub stream: StreamId,
    pub offset: Offset,
    pub timestamp: Timestamp,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishResponse {
    pub data: Vec<PublishResponseKey>,
}

#[derive(Debug, Copy, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Compression {
    None,
    Deflate,
}

#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotData {
    compression: Compression,
    data: Binary,
}

impl SnapshotData {
    pub fn new(compression: Compression, data: impl Into<Box<[u8]>>) -> Self {
        Self {
            compression,
            data: data.into().into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum StartFrom {
    Offsets(OffsetMap),
    // Snapshot {
    //     compression: std::collections::BTreeSet<Compression>,
    // },
}

impl StartFrom {
    pub fn min_offsets(&self) -> OffsetMap {
        match self {
            StartFrom::Offsets(o) => o.clone(),
            // _ => OffsetMap::empty(),
        }
    }
}

/// The session identifier used in /subscribe_monotonic
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SessionId(Box<str>);

impl Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&*self.0)
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
        &*self.0
    }
}

/// Subscribe to live updates as the Event Services receives or publishes new events,
/// until the recipient would need to time travel
///
/// Time travel is defined as receiving an event that needs to be sorted earlier than
/// an event that has already been received.
///
/// Send this structure to the `$BASE_URI/subscribe` endpoint to retrieve an
/// unbounded stream of events. If the lower bound is given, it filters out all
/// events that are included in the `lower_bound` OffsetMap.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeMonotonicRequest {
    /// This id uniquely identifies one particular session. Connecting again with this
    /// SessionId shall only be done after a TimeTravel message has been received. The
    /// subscription is stored with the Session and all previous state is destroyed
    /// upon receiving a different subscription for this session.
    pub session: SessionId,
    /// Definition of the events to be received by this session, i.e. a selection of
    /// tags coupled with other flags like “is local”.
    pub query: Query,
    /// The consumer may already have kept state and know at which point to resume a
    /// previously interrupted stream. In this case, StartFrom::Offsets is used,
    /// otherwise StartFrom::Snapshot indicates that the PondService shall figure
    /// out where best to start out from, possibly sending a `State` message first.
    #[serde(flatten)]
    pub from: StartFrom,
}

/// Response to subscribeMonotonic is a stream of events terminated by a time travel
#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SubscribeMonotonicResponse {
    /// This message may be sent in the beginning when a suitable snapshot has been
    /// found for this session. It may also be sent at later times when suitable
    /// snapshots become available by other means (if for example this session is
    /// computed also on a different node).
    #[serde(rename_all = "camelCase")]
    State { snapshot: SnapshotData },
    /// This is the main message, a new event that is to be applied directly to the
    /// currently known state to produce the next state.
    #[serde(rename_all = "camelCase")]
    Event {
        #[serde(flatten)]
        event: EventResponse<Payload>,
        caught_up: bool,
    },
    /// This message ends the stream in case a replay becomes necessary due to
    /// time travel. The contained event key signals how far back the replay will
    /// reach so that the consumer can invalidate locally stored snapshots (if
    /// relevant).
    #[serde(rename_all = "camelCase")]
    TimeTravel { new_start: EventKey },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum QueryResponse {
    #[serde(rename_all = "camelCase")]
    Event(EventResponse<Payload>),
    // #[serde(rename_all = "camelCase")]
    // Offset(OffsetMap),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SubscribeResponse {
    #[serde(rename_all = "camelCase")]
    Event(EventResponse<Payload>),
    // #[serde(rename_all = "camelCase")]
    // Offset(OffsetMap),
}

/// Response to the `offsets` endpoint
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OffsetsResponse {
    /// Currently validated [`OffsetMap`] locally available
    pub present: OffsetMap,
    /// Number of events per [`StreamId`] pending replication to this node
    pub to_replicate: BTreeMap<StreamId, NonZeroU64>,
}

#[async_trait]
pub trait EventService: Clone + Send {
    async fn offsets(&self) -> Result<OffsetsResponse>;

    async fn publish(&self, request: PublishRequest) -> Result<PublishResponse>;

    async fn query(&self, request: QueryRequest) -> Result<BoxStream<'static, QueryResponse>>;

    async fn subscribe(&self, request: SubscribeRequest) -> Result<BoxStream<'static, SubscribeResponse>>;

    async fn subscribe_monotonic(
        &self,
        request: SubscribeMonotonicRequest,
    ) -> Result<BoxStream<'static, SubscribeMonotonicResponse>>;
}
