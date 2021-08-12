use actyx_sdk::{
    service::{Diagnostic, EventResponse},
    NodeId, Payload,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub peer_id: String,
    pub addr: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    pub peer_id: String,
    pub addrs: Vec<String>,
}

// inlined from https://github.com/Actyx/Cosmos/blob/master/rust/actyx/node-manager-bindings/src/types.rs
use actyx_sdk::service::OffsetsResponse;
use util::formats::NodesInspectResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectedNodeDetails {
    pub node_id: NodeId,
    pub display_name: String,
    pub started_iso: String,
    pub started_unix: i64,
    pub version: String,
    pub addrs: String,
    pub settings: serde_json::Value,
    pub settings_schema: serde_json::Value,
    pub swarm_state: NodesInspectResponse,
    pub offsets: Option<OffsetsResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
#[allow(clippy::enum_variant_names)]
pub enum Node {
    ReachableNode {
        addr: String,
        details: ConnectedNodeDetails,
    },
    UnauthorizedNode {
        addr: String,
    },
    UnreachableNode {
        addr: String,
    },
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
