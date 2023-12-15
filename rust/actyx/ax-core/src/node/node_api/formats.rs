use crate::util::formats::{ActyxOSResult, NodesLsResponse};
use ax_types::NodeId;
use tokio::sync::oneshot::Sender;

#[derive(Debug)]
pub enum NodesRequest {
    Ls(Sender<ActyxOSResult<NodesLsResponse>>),
    GetNodeId(Sender<ActyxOSResult<NodeId>>),
}
