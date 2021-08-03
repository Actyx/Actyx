use actyx_sdk::{service::OffsetsResponse, NodeId};
use serde::{Deserialize, Serialize};
use util::formats::NodesInspectResponse;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Nothing {}

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
