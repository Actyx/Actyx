use super::{OffsetMap, Payload};
use crate::{
    tagged::{EventKey, StreamId, TagSet},
    LamportTimestamp, Offset, TimeStamp,
};
use serde::{Deserialize, Serialize};

/// The order in which you want to receive events for a query
///
/// Event streams can be request with different ordering requirements from the
/// Event Service:
///
///  - in strict forward Lamport order
///  - in strict backwards Lamport order (only possible when requesting with an upper bound OffsetMap)
///  - ordered in forward order per source (ActyxOS node), but not between sources
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum Order {
    /// Events are sorted by ascending Lamport timestamp and source ID, which defines a
    /// total order. If the subscription does not restrict the set of source
    /// IDs then a new source appearing with old events will lead to these old
    /// events only being delivered if they are younger than the youngest already
    /// delivered event.
    ///
    /// Requesting this order will stall the stream’s delivery while one of the contained
    /// sources stops sending events (for example when it goes offline or is destroyed).
    Asc,
    /// Events are sorted by descending Lamport timestamp and descending source ID,
    /// which is the exact reverse of the `Lamport` ordering. Requests with this
    /// ordering will only be successful if they include an upper bound OffsetMap
    /// and if that map is less than or equal to the OffsetMap obtained with
    /// the `get_offsets` method.
    Desc,
    /// Events are sorted within each stream by ascending Lamport timestamp, with streams
    /// from different sources interleaved in an undefined order.
    ///
    /// This is the preferred ordering for live streams as it permits new information
    /// to be made available as soon as it is delivered to the ActyxOS node, without
    /// needing to wait for all other sources to confirm the ordering first.
    StreamAsc,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryApiRequest {
    pub lower_bound: Option<OffsetMap>,
    pub upper_bound: OffsetMap,
    pub r#where: String, // TODO: (de-)serialize to/from TagExpr?
    pub order: Order,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeApiRequest {
    pub offsets: Option<OffsetMap>,
    pub r#where: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventResponse<T> {
    pub stream: StreamId,
    pub lamport: LamportTimestamp,
    pub offset: Offset,
    pub timestamp: TimeStamp,
    pub tags: TagSet,
    pub payload: T,
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
        let payload = self.payload.extract::<T>()?;
        let EventResponse {
            stream,
            lamport,
            offset,
            timestamp,
            tags,
            ..
        } = self.clone();
        Ok(EventResponse {
            stream,
            lamport,
            offset,
            timestamp,
            tags,
            payload,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishApiRequestElement {
    pub tags: TagSet,
    pub payload: Payload,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishApiRequest {
    pub data: Vec<PublishApiRequestElement>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishApiResponse {
    pub data: Vec<EventKey>,
}
