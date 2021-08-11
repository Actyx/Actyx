use crate::errors::ActyxOSResult;
use actyx_sdk::NodeId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeVersion {
    pub profile: String,
    pub target: String,
    pub version: String,
    pub git_hash: String,
}

impl std::fmt::Display for NodeVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}-{}-{}-{}",
            self.version, self.git_hash, self.target, self.profile
        ))
    }
}
#[derive(Clone, Debug)]
pub struct AdminProtocol();

impl libp2p_streaming_response::Codec for AdminProtocol {
    type Request = AdminRequest;
    type Response = ActyxOSResult<AdminResponse>;
    fn protocol_info() -> &'static [u8] {
        b"/actyx/admin/1.0.0"
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AdminRequest {
    NodesLs,
    NodesInspect,
    NodesShutdown,
    SettingsGet {
        scope: String,
        no_defaults: bool,
    },
    SettingsSet {
        scope: String,
        json: serde_json::Value,
        ignore_errors: bool,
    },
    SettingsSchema {
        scope: String,
    },
    SettingsScopes,
    SettingsUnset {
        scope: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AdminResponse {
    NodesLsResponse(NodesLsResponse),
    NodesInspectResponse(NodesInspectResponse),
    SettingsGetResponse(serde_json::Value),
    SettingsSetResponse(serde_json::Value),
    SettingsSchemaResponse(serde_json::Value),
    SettingsScopesResponse(Vec<String>),
    SettingsUnsetResponse,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodesLsResponse {
    pub node_id: NodeId,
    pub display_name: String,
    pub started_iso: String,
    pub started_unix: i64,
    pub version: NodeVersion,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct SetSettingsRequest {
    pub settings: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodesInspectResponse {
    pub peer_id: String,
    pub swarm_addrs: Vec<String>,
    pub announce_addrs: Vec<String>,
    pub admin_addrs: Vec<String>,
    pub connections: Vec<Connection>,
    pub known_peers: Vec<Peer>,
}

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
