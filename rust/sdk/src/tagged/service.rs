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

use super::{event::EventKey, Compression, Event, NodeId, SessionId, SnapshotData};
use crate::{OffsetMap, Payload};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum StartFrom {
    Offsets(OffsetMap),
    Snapshot { compression: BTreeSet<Compression> },
}

impl StartFrom {
    pub fn min_offsets(&self) -> OffsetMap {
        if let StartFrom::Offsets(o) = self {
            o.clone()
        } else {
            OffsetMap::empty()
        }
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
#[derive(Debug, Serialize, Deserialize, Clone, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeUntilTimeTravelApiRequest {
    pub session: SessionId,
    pub subscription: String,
    #[serde(flatten)]
    pub from: StartFrom,
}

/// Response to subscribeUntilTimeTravel is a stream of events possibly preceded by a
/// start message and terminated by a time travel
///
///
#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SubscribeUntilTimeTravelResponse {
    #[serde(rename_all = "camelCase")]
    Start { snapshot: SnapshotData },
    #[serde(rename_all = "camelCase")]
    Event {
        #[serde(flatten)]
        event: Event<Payload>,
        caught_up: bool,
    },
    #[serde(rename_all = "camelCase")]
    TimeTravel { new_start: EventKey },
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
    use crate::{
        source_id,
        tagged::{Metadata, TagSet},
        LamportTimestamp, Offset, TimeStamp,
    };

    #[test]
    fn must_serialize_subscribe_until_time_travel() {
        let req = SubscribeUntilTimeTravelApiRequest {
            session: "sess".into(),
            subscription: "'tagA' & 'tagB'".to_owned(),
            from: StartFrom::Offsets(OffsetMap::empty()),
        };
        let s = serde_json::to_string(&req).unwrap();
        assert_eq!(
            s,
            r#"{"session":"sess","subscription":"'tagA' & 'tagB'","offsets":{}}"#
        );
        let r: SubscribeUntilTimeTravelApiRequest = serde_json::from_str(&*s).unwrap();
        assert_eq!(r, req);

        let req = SubscribeUntilTimeTravelApiRequest {
            session: "sess".into(),
            subscription: "'tagA' & 'tagB'".to_owned(),
            from: StartFrom::Snapshot {
                compression: [Compression::Deflate].iter().copied().collect(),
            },
        };
        let s = serde_json::to_string(&req).unwrap();
        assert_eq!(
            s,
            r#"{"session":"sess","subscription":"'tagA' & 'tagB'","snapshot":{"compression":["deflate"]}}"#
        );
        let r: SubscribeUntilTimeTravelApiRequest = serde_json::from_str(&*s).unwrap();
        assert_eq!(r, req);

        let resp = SubscribeUntilTimeTravelResponse::Start {
            snapshot: SnapshotData::new(Compression::None, &[1, 2, 3][..]),
        };
        let s = serde_json::to_string(&resp).unwrap();
        assert_eq!(
            s,
            r#"{"type":"start","snapshot":{"compression":"none","data":"AQID"}}"#
        );
        let r: SubscribeUntilTimeTravelResponse = serde_json::from_str(&*s).unwrap();
        assert_eq!(r, resp);

        let resp = SubscribeUntilTimeTravelResponse::Event {
            event: Event {
                key: EventKey {
                    lamport: LamportTimestamp::new(1),
                    stream: source_id!("src").into(),
                    offset: Offset::mk_test(3),
                },
                meta: Metadata {
                    timestamp: TimeStamp::new(2),
                    tags: TagSet::empty(),
                },
                payload: Payload::default(),
            },
            caught_up: true,
        };
        let s = serde_json::to_string(&resp).unwrap();
        assert_eq!(
            s,
            r#"{"type":"event","key":{"lamport":1,"stream":"src","offset":3},"meta":{"timestamp":2,"tags":[]},"payload":null,"caughtUp":true}"#
        );
        let r: SubscribeUntilTimeTravelResponse = serde_json::from_str(&*s).unwrap();
        assert_eq!(r, resp);

        let resp = SubscribeUntilTimeTravelResponse::TimeTravel {
            new_start: EventKey::default(),
        };
        let s = serde_json::to_string(&resp).unwrap();
        assert_eq!(
            s,
            r#"{"type":"timeTravel","newStart":{"lamport":0,"stream":"!","offset":0}}"#
        );
        let r: SubscribeUntilTimeTravelResponse = serde_json::from_str(&*s).unwrap();
        assert_eq!(r, resp);
    }
}
