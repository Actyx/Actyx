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
//! Data types needed for interacting with the [ActyxOS Event Service](https://developer.actyx.com/docs/os/api/event-service/), plus an optional HTTP client binding
//!
//! The [`EventService`](struct.EventService.html) client is only available under the `client` feature flag.

use crate::{
    arcval_scalar,
    event::{EventKey, FishName, OffsetMap, Payload, Semantics, SourceId},
    tagged,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::{EventService, EventServiceError};

arcval_scalar! {
    /// The session identifier used in subscribeUntilTimeTravel
    #[derive(Serialize, Deserialize)]
    pub struct SessionId(str);
}

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
    Lamport,
    /// Events are sorted by descending Lamport timestamp and descending source ID,
    /// which is the exact reverse of the `Lamport` ordering. Requests with this
    /// ordering will only be successful if they include an upper bound OffsetMap
    /// and if that map is less than or equal to the OffsetMap obtained with
    /// the `get_offsets` method.
    LamportReverse,
    /// Events are sorted within each stream by ascending Lamport timestamp, with streams
    /// from different sources interleaved in an undefined order.
    ///
    /// This is the preferred ordering for live streams as it permits new information
    /// to be made available as soon as it is delivered to the ActyxOS node, without
    /// needing to wait for all other sources to confirm the ordering first.
    SourceOrdered,
}

/// A subscription describes a selection of events.
///
/// It is based on the characteristics of
///
///  - semantics (i.e. the kind of fish when using the Pond)
///  - name (i.e. the particular instance of this kind of fish)
///  - source ID (i.e. the originating ActyxOS node)
#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(from = "SubscriptionOnWire")]
pub struct Subscription {
    semantics: Option<Semantics>,
    name: Option<FishName>,
    source: Option<SourceId>,
}

// canonicalize: empty string is the same as absent
impl From<SubscriptionOnWire> for Subscription {
    fn from(other: SubscriptionOnWire) -> Self {
        Self {
            semantics: other.semantics.and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(Semantics::new(s).unwrap())
                }
            }),
            name: other.name.and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(FishName::new(s).unwrap())
                }
            }),
            source: other.source.and_then(|s| {
                if s.is_empty() {
                    None
                } else {
                    Some(SourceId::new(s).unwrap())
                }
            }),
        }
    }
}

#[derive(Deserialize)]
struct SubscriptionOnWire {
    pub semantics: Option<String>,
    pub name: Option<String>,
    pub source: Option<String>,
}

impl Subscription {
    /// Subscribe to all events accessible to this app. This set can be very big,
    /// it is recommended to structure events into smaller streams and request those.
    pub fn all_events() -> Self {
        Self {
            semantics: None,
            name: None,
            source: None,
        }
    }

    /// Subscribe to all events of the given semantics, regardless of which fish
    /// instance produced them or where.
    pub fn wildcard(semantics: Semantics) -> Self {
        Self {
            semantics: Some(semantics),
            name: None,
            source: None,
        }
    }

    /// Subscribe to all events of a distributed fish, identified by its semantics
    /// and name.
    pub fn distributed(semantics: Semantics, name: FishName) -> Self {
        Self {
            semantics: Some(semantics),
            name: Some(name),
            source: None,
        }
    }

    /// Subscribe to precisely a single fish on the given ActyxOS node.
    pub fn local(semantics: Semantics, name: FishName, source: SourceId) -> Self {
        Self {
            semantics: Some(semantics),
            name: Some(name),
            source: Some(source),
        }
    }

    pub fn as_tuple(&self) -> (Option<Semantics>, Option<FishName>, Option<SourceId>) {
        (self.semantics.clone(), self.name.clone(), self.source)
    }
}

/// Query the Event Service for events it has already stored
///
/// Send this structure to the `$BASE_URI/query` endpoint to retrieve a bounded
/// stream of events between the lower and upper bounds. An absent lower bound
/// includes all events from the beginning, otherwise it excludes all events included
/// within the `lower_bound` OffsetMap.
///
/// The order of events is specified independently, i.e. if you ask for
/// LamportReverse order you’ll get the events starting with `upper_bound` and
/// going backwards down to `lower_bound`.
///
/// The delivered event stream will be filtered by the subscriptions: an event
/// is included if any of the subscriptions matches.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QueryApiRequest {
    pub lower_bound: Option<OffsetMap>,
    pub upper_bound: OffsetMap,
    pub subscriptions: Vec<Subscription>,
    pub order: Order,
}

/// Subscribe to live updates as the Event Service receives or publishes new events
///
/// Send this structure to the `$BASE_URI/subscribe` endpoint to retrieve an
/// unbounded stream of events. If the lower bound is given, it filters out all
/// events that are included in the `lower_bound` OffsetMap.
///
/// The common pattern is to take note of consumed events by adding them into an
/// OffsetMap and resuming the stream from this OffsetMap after an app restart.
///
/// The delivered event stream will be filtered by the subscriptions: an event
/// is included if any of the subscriptions matches.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeApiRequest {
    pub lower_bound: Option<OffsetMap>,
    pub subscriptions: Vec<Subscription>,
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
///
/// The common pattern is to take note of consumed events by adding them into an
/// OffsetMap and resuming the stream from this OffsetMap after an app restart.
///
/// The delivered event stream will be filtered by the subscriptions: an event
/// is included if any of the subscriptions matches.
#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeUntilTimeTravelApiRequest {
    pub lower_bound: Option<OffsetMap>,
    pub subscriptions: Vec<Subscription>,
    pub session: Option<SessionId>,
}

/// Response to subscribeUntilTimeTravel is a stream of events terminated by a time travel.
#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SubscribeUntilTimeTravelResponse {
    Event(tagged::Event<Payload>),
    #[serde(rename_all = "camelCase")]
    TimeTravel {
        session: SessionId,
        new_start: EventKey,
    },
}

/// The structure of a single event to be published
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishEvent {
    pub semantics: Semantics,
    pub name: FishName,
    pub payload: Payload,
}

/// Ask the Event Service to publish a sequence of events
///
/// Send this structure to the `$BASE_URI/publish` endpoint to publish a sequence
/// of events in the given order with their respective semantics and names.
///
/// The `payload` member of the `PublishEvent` is most conveniently serialized
/// using the `Payload::compact` method.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublishRequestBody {
    pub data: Vec<PublishEvent>,
}

/// Response to the `node_id` endpoint
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeIdResponse {
    pub node_id: SourceId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::{LamportTimestamp, Offset, TimeStamp},
        fish_name, semantics, source_id,
        tagged::Metadata,
    };

    #[test]
    fn must_pick_up_subscription() {
        let sub = Subscription::local(
            semantics!("semantics"),
            fish_name!("name"),
            source_id!("source"),
        );
        let bytes = serde_json::to_string(&sub).unwrap();
        assert_eq!(
            bytes,
            r#"{"semantics":"semantics","name":"name","source":"source"}"#.to_owned()
        );
    }

    #[test]
    fn must_pick_up_subscription_set() {
        let bytes = r#"{"source":""}"#;
        let subs: Subscription = serde_json::from_str(bytes).unwrap();
        assert_eq!(subs.as_tuple(), (None, None, None));

        let bytes = r#"{"name":""}"#;
        let subs: Subscription = serde_json::from_str(bytes).unwrap();
        assert_eq!(subs.as_tuple(), (None, None, None));

        let bytes = r#"{"name":"name"}"#;
        let subs: Subscription = serde_json::from_str(bytes).unwrap();
        assert_eq!(subs.as_tuple(), (None, Some(fish_name!("name")), None));

        let bytes = r#"{"source":"name"}"#;
        let subs: Subscription = serde_json::from_str(bytes).unwrap();
        assert_eq!(subs.as_tuple(), (None, None, Some(source_id!("name"))));
    }

    #[test]
    fn must_pick_up_subscription_set_owned() {
        let bytes = br#"{"source":""}"#.as_ref();
        let subs: Subscription = serde_json::from_reader(bytes).unwrap();
        assert_eq!(subs.as_tuple(), (None, None, None));

        let bytes = br#"{"name":""}"#.as_ref();
        let subs: Subscription = serde_json::from_reader(bytes).unwrap();
        assert_eq!(subs.as_tuple(), (None, None, None));

        let bytes = br#"{"name":"name"}"#.as_ref();
        let subs: Subscription = serde_json::from_reader(bytes).unwrap();
        assert_eq!(subs.as_tuple(), (None, Some(fish_name!("name")), None));

        let bytes = br#"{"source":"name"}"#.as_ref();
        let subs: Subscription = serde_json::from_reader(bytes).unwrap();
        assert_eq!(subs.as_tuple(), (None, None, Some(source_id!("name"))));
    }

    #[test]
    fn must_serialize_subscribe_until_time_travel() {
        let req = SubscribeUntilTimeTravelApiRequest {
            lower_bound: None,
            subscriptions: vec![],
            session: None,
        };
        let s = serde_json::to_string(&req).unwrap();
        assert_eq!(
            s,
            r#"{"lowerBound":null,"subscriptions":[],"session":null}"#
        );
        let r: SubscribeUntilTimeTravelApiRequest = serde_json::from_str(&*s).unwrap();
        assert_eq!(r, req);

        let resp = SubscribeUntilTimeTravelResponse::Event(tagged::Event {
            key: EventKey {
                lamport: LamportTimestamp::new(1),
                source: source_id!("src"),
                offset: Offset(3),
            },
            meta: Metadata {
                timestamp: TimeStamp::new(2),
                tags: Vec::new(),
            },
            payload: Payload::default(),
        });
        let s = serde_json::to_string(&resp).unwrap();
        assert_eq!(
            s,
            r#"{"type":"event","key":{"lamport":1,"source":"src","offset":3},"meta":{"timestamp":2,"tags":[]},"payload":null}"#
        );
        let r: SubscribeUntilTimeTravelResponse = serde_json::from_str(&*s).unwrap();
        assert_eq!(r, resp);

        let resp = SubscribeUntilTimeTravelResponse::TimeTravel {
            session: SessionId::from("session"),
            new_start: EventKey::default(),
        };
        let s = serde_json::to_string(&resp).unwrap();
        assert_eq!(
            s,
            r#"{"type":"timeTravel","session":"session","newStart":{"lamport":0,"source":"\u0000","offset":-1}}"#
        );
        let r: SubscribeUntilTimeTravelResponse = serde_json::from_str(&*s).unwrap();
        assert_eq!(r, resp);
    }
}
