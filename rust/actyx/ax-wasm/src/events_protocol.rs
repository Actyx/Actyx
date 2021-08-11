use actyx_sdk::{
    service::{
        Diagnostic, EventResponse, OffsetsResponse, PublishRequest, PublishResponse, QueryRequest,
        SubscribeMonotonicRequest, SubscribeRequest,
    },
    OffsetMap, Payload,
};
use libp2p_streaming_response::Codec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct EventsProtocol;

impl Codec for EventsProtocol {
    type Request = EventsRequest;
    type Response = EventsResponse;

    fn protocol_info() -> &'static [u8] {
        b"/actyx/events/v2"
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventDiagnostic {
    Event(EventResponse<Payload>),
    Diagnostic(Diagnostic),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeManagerEventsRes {
    pub events: Option<Vec<EventDiagnostic>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EventsRequest {
    Offsets,
    Query(QueryRequest),
    Subscribe(SubscribeRequest),
    SubscribeMonotonic(SubscribeMonotonicRequest),
    Publish(PublishRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EventsResponse {
    Error {
        message: String,
    },
    Offsets(OffsetsResponse),
    Event(EventResponse<Payload>),
    OffsetMap {
        offsets: OffsetMap,
    },
    Publish(PublishResponse),
    Diagnostic(Diagnostic),
    #[serde(other)]
    FutureCompat,
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyx_sdk::{
        app_id,
        service::{Severity, StartFrom},
        tags, Event, EventKey, LamportTimestamp, Metadata, NodeId, Offset, Timestamp,
    };
    use std::collections::BTreeMap;

    fn req(req: EventsRequest) -> String {
        serde_json::to_string(&req).unwrap()
    }

    fn res(res: EventsResponse) -> String {
        serde_json::to_string(&res).unwrap()
    }

    #[test]
    fn requests() {
        assert_eq!(req(EventsRequest::Offsets), r#"{"type":"offsets"}"#);
        assert_eq!(
            req(EventsRequest::Query(QueryRequest {
                lower_bound: None,
                upper_bound: None,
                query: "FROM allEvents".parse().unwrap(),
                order: actyx_sdk::service::Order::Asc
            })),
            r#"{"type":"query","lowerBound":null,"upperBound":null,"query":"FROM allEvents END","order":"asc"}"#
        );
        assert_eq!(
            req(EventsRequest::Subscribe(SubscribeRequest {
                lower_bound: None,
                query: "FROM allEvents".parse().unwrap(),
            })),
            r#"{"type":"subscribe","lowerBound":null,"query":"FROM allEvents END"}"#
        );
        assert_eq!(
            req(EventsRequest::SubscribeMonotonic(SubscribeMonotonicRequest {
                session: "".into(),
                from: StartFrom::LowerBound(OffsetMap::default()),
                query: "FROM allEvents".parse().unwrap(),
            })),
            r#"{"type":"subscribeMonotonic","session":"","query":"FROM allEvents END","lowerBound":{}}"#
        );
    }

    fn ev(n: u32) -> EventResponse<Payload> {
        let key = EventKey {
            lamport: LamportTimestamp::default(),
            stream: NodeId::from_bytes(&[0; 32]).unwrap().stream(0.into()),
            offset: Offset::default(),
        };
        let meta = Metadata {
            timestamp: Timestamp::new(12),
            tags: tags!("a", "b"),
            app_id: app_id!("app"),
        };
        let payload = Payload::from_json_str(&format!("{}", n)).unwrap();
        Event { key, meta, payload }.into()
    }

    #[test]
    fn responses() {
        assert_eq!(
            res(EventsResponse::Error { message: "haha".into() }),
            r#"{"type":"error","message":"haha"}"#
        );
        assert_eq!(
            res(EventsResponse::Event(ev(3))),
            r#"{"type":"event","lamport":0,"stream":"...........................................-0","offset":0,"timestamp":12,"tags":["a","b"],"appId":"app","payload":3}"#
        );
        assert_eq!(
            res(EventsResponse::Diagnostic(Diagnostic {
                severity: Severity::Warning,
                message: "buh".to_owned()
            })),
            r#"{"type":"diagnostic","severity":"warning","message":"buh"}"#
        );
        assert_eq!(
            serde_json::from_str::<EventsResponse>(r#"{"type":"diagnostic","severity":"warning","message":"buh"}"#)
                .unwrap(),
            EventsResponse::Diagnostic(Diagnostic {
                severity: Severity::Warning,
                message: "buh".to_owned()
            })
        );
        assert_eq!(
            res(EventsResponse::OffsetMap {
                offsets: OffsetMap::default()
            }),
            r#"{"type":"offsetMap","offsets":{}}"#
        );
        assert_eq!(
            res(EventsResponse::Offsets(OffsetsResponse {
                present: OffsetMap::default(),
                to_replicate: BTreeMap::default()
            })),
            r#"{"type":"offsets","present":{},"toReplicate":{}}"#
        );
        assert_eq!(
            res(EventsResponse::Publish(PublishResponse { data: vec![] })),
            r#"{"type":"publish","data":[]}"#
        );
    }

    #[test]
    fn future_compat() {
        assert_eq!(
            serde_json::from_str::<EventsResponse>(r#"{"type":"fromTheFuture","a":null}"#).unwrap(),
            EventsResponse::FutureCompat,
        );
    }
}
