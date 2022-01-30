use crate::{components::ComponentType, node::NodeError, node_api::formats::NodesRequest, settings::SettingsRequest};
use actyx_sdk::NodeId;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use util::formats::NodeName;

pub mod node_settings;
pub use node_settings::Settings;

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct NodeDetails {
    pub node_id: NodeId,
    pub node_name: NodeName,
}
impl NodeDetails {
    pub fn from_settings(settings: &Settings, node_id: NodeId) -> Self {
        Self {
            node_id,
            node_name: NodeName(settings.admin.display_name.clone()),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum Entity {
    Host,
    HostUi,
    // ActyxCli,
    // Node,
}

pub enum ExternalEvent {
    NodesRequest(NodesRequest),
    SettingsRequest(SettingsRequest),
    RestartRequest(ComponentType),
    ShutdownRequested(ShutdownReason),
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct NodeState {
    pub details: NodeDetails,
    // This stores only the settings at scope com.actyx
    pub settings: Settings,
    pub started_at: DateTime<Utc>,
}
impl NodeState {
    pub fn new(node_id: NodeId, settings: Settings) -> Self {
        let details = NodeDetails::from_settings(&settings, node_id);

        Self {
            settings,
            details,
            started_at: Utc::now(),
        }
    }
}
#[derive(Debug, Clone)]
pub enum ShutdownReason {
    TriggeredByHost,
    TriggeredByUser,
    Internal(NodeError),
}
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum NodeEvent {
    StateUpdate(NodeState),
    Shutdown(ShutdownReason),
}

pub(crate) trait ResultInspect<T, E> {
    fn inspect_err<F>(self, f: F) -> Self
    where
        F: FnMut(&E);
}
impl<T, E> ResultInspect<T, E> for Result<T, E> {
    fn inspect_err<F>(self, mut f: F) -> Self
    where
        F: FnMut(&E),
    {
        if let Err(ref e) = self {
            f(e)
        };
        self
    }
}
