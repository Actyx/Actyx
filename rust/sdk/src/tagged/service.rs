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

use super::{event::EventKey, Event};
use crate::{OffsetMap, Payload, SourceId};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// The session identifier used in subscribeUntilTimeTravel
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

/// The ActyxOS node identifier
///
/// Each node may emit multiple sources, each identified by its own [`SourceId`](../struct.SourceId.html).
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct NodeId(Box<str>);

impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&*self.0)
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl NodeId {
    /// Check whether the given `SourceId` is published by the ActyxOS node identified by this `NodeId`
    ///
    /// This is accomplished without accessing further data by deriving the `SourceId` from its node’s
    /// `NodeId`: the `NodeId` is extended with an underscore and possibly more characters to obtain
    /// the `SourceId`.
    pub fn has_source_id(&self, source_id: SourceId) -> bool {
        source_id.as_str().len() > self.0.len()
            && source_id.as_str().starts_with(&*self.0)
            && source_id.as_str().as_bytes()[self.0.len()] == b'_'
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
    pub subscription: String,
    pub session: Option<SessionId>,
}

/// Response to subscribeUntilTimeTravel is a stream of events terminated by a time travel.
#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SubscribeUntilTimeTravelResponse {
    Event(Event<Payload>),
    #[serde(rename_all = "camelCase")]
    TimeTravel {
        session: SessionId,
        new_start: EventKey,
    },
}

/// Response to the `node_id` endpoint
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeIdResponse {
    pub node_id: NodeId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{source_id, tagged::Metadata, LamportTimestamp, Offset, TimeStamp};

    #[test]
    fn must_serialize_subscribe_until_time_travel() {
        let req = SubscribeUntilTimeTravelApiRequest {
            lower_bound: None,
            subscription: "'tagA' & 'tagB'".to_owned(),
            session: None,
        };
        let s = serde_json::to_string(&req).unwrap();
        assert_eq!(
            s,
            r#"{"lowerBound":null,"subscription":"'tagA' & 'tagB'","session":null}"#
        );
        let r: SubscribeUntilTimeTravelApiRequest = serde_json::from_str(&*s).unwrap();
        assert_eq!(r, req);

        let resp = SubscribeUntilTimeTravelResponse::Event(Event {
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
