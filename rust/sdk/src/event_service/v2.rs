use super::{OffsetMap, Payload};
use crate::tagged::{EventKey, TagSet};
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
    pub r#where: String,
    pub order: Order,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeApiRequest {
    pub offsets: Option<OffsetMap>,
    pub subscription: String,
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
