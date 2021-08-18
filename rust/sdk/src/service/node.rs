use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Node Information
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfoResponse {
    /// Number of currently connected nodes
    pub connected_nodes: usize,
    /// Uptime of the node
    pub uptime: Duration,
    /// Version string of the node
    pub version: String,
}
