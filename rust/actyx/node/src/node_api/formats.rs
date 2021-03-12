use actyxos_lib::{ActyxOSResult, NodesLsResponse};
use actyxos_sdk::tagged::NodeId;
use tokio::sync::oneshot::Sender;
#[derive(Debug)]
pub enum NodesRequest {
    Ls(Sender<ActyxOSResult<NodesLsResponse>>),
    GetNodeId(Sender<ActyxOSResult<NodeId>>),
}

// TODO: create token
