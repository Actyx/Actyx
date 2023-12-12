use crate::NodeId;
use im::OrdMap;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Node Information
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfoResponse {
    /// Number of currently connected nodes
    pub connected_nodes: usize,
    /// Uptime of the node
    pub uptime: Duration,
    /// Version string of the node
    pub version: String,
    /// Swarm status as seen by the node
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub swarm_state: Option<SwarmState>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmState {
    pub peers_status: OrdMap<NodeId, PeerStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    /// Acknowledge replication within two gossip cycles
    LowLatency,
    /// Acknowledge replication within five gossip cycles
    HighLatency,
    /// Acknowledge replication of at least half of all streams within
    /// five gossip cycles
    PartiallyWorking,
    /// Acknowledge replication of less than half of all streams within
    /// five gossip cycles
    NotWorking,
}
