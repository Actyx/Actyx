use super::ActyxOSResult;
use crate::util::version::NodeVersion;
use ax_sdk::NodeId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct AdminProtocol();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogQueryMode {
    All,
    MostRecent {
        count: usize,
    },
    ByTime {
        since: DateTime<Utc>,
        to: Option<DateTime<Utc>>, // None eq. now if follow == false
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQuery {
    pub mode: LogQueryMode,
    pub follow: bool,
}

impl crate::libp2p_streaming_response::Codec for AdminProtocol {
    type Request = AdminRequest;
    type Response = ActyxOSResult<AdminResponse>;

    fn info_v1() -> &'static str {
        "/actyx/admin/1.0.0"
    }

    fn info_v2() -> &'static [&'static str] {
        &["/actyx/admin/1.2", "/actyx/admin/1.1"]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AdminRequest {
    NodesLs,
    NodesInspect,
    NodesShutdown,
    SettingsGet {
        scope: crate::settings::Scope,
        no_defaults: bool,
    },
    SettingsSet {
        scope: crate::settings::Scope,
        json: serde_json::Value,
        ignore_errors: bool,
    },
    SettingsSchema {
        scope: crate::settings::Scope,
    },
    SettingsScopes,
    SettingsUnset {
        scope: crate::settings::Scope,
    },
    /// List all the existing topics in the nodes
    TopicLs,
    /// Delete the given topic from all nodes
    TopicDelete {
        name: String,
    },
    // Without this, the request isn't processed and the client times out
    #[serde(other)]
    FutureCompat,
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
    TopicLsResponse(TopicLsResponse),
    TopicDeleteResponse(TopicDeleteResponse),
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
    #[serde(default)]
    pub since: String,
    #[serde(default)]
    pub outbound: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    pub peer_id: String,
    #[serde(default)]
    pub info: PeerInfo,
    pub addrs: Vec<String>,
    #[serde(default)]
    pub addr_source: Vec<String>,
    #[serde(default)]
    pub addr_since: Vec<String>,
    #[serde(default)]
    pub failures: Vec<Failure>,
    pub ping_stats: Option<PingStats>,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PeerInfo {
    pub protocol_version: Option<String>,
    pub agent_version: Option<String>,
    pub protocols: Vec<String>,
    pub listeners: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub addr: String,
    pub time: String,
    pub display: String,
    pub details: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PingStats {
    pub current: u32,
    pub decay_3: u32,
    pub decay_10: u32,
    pub failures: u32,
    pub failure_rate: u32,
}

type TopicName = String;
type TopicSize = u64;

/// Request for the list of topics in a node.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TopicLsRequest;

/// Response for the list of topics in a node.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TopicLsResponse {
    pub node_id: NodeId,
    pub active_topic: TopicName,
    pub topics: BTreeMap<TopicName, TopicSize>,
}

/// Request to delete a given topic in a node.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TopicDeleteRequest {
    pub topic: String,
}

/// Response to the deletion of a topic in a node.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TopicDeleteResponse {
    pub node_id: NodeId,
    /// True if any file was deleted.
    pub deleted: bool,
}
