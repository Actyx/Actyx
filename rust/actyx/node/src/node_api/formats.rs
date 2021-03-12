use actyxos_sdk::NodeId;
use tokio::sync::oneshot::Sender;
use util::formats::{ActyxOSResult, NodesLsResponse};
#[derive(Debug)]
pub enum NodesRequest {
    Ls(Sender<ActyxOSResult<NodesLsResponse>>),
    GetNodeId(Sender<ActyxOSResult<NodeId>>),
}

// TODO: create token
