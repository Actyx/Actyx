use actyxos_sdk::{service::NodeIdResponse, AppId, NodeId};
use warp::*;

use crate::{
    util::{
        filters::{accept_json, header_token},
        reject, Result,
    },
    NodeInfo,
};

fn with_node_id(node_id: NodeId) -> impl Filter<Extract = (NodeId,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || node_id)
}

pub(crate) fn route(node_info: NodeInfo) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let node_id = node_info.node_id;
    let auth = crate::util::filters::authenticate(node_info, header_token());
    auth.and(get())
        .and(accept_json())
        .and(with_node_id(node_id))
        .and_then(handle)
}

async fn handle(_app_id: AppId, node_id: NodeId) -> Result<impl Reply> {
    Ok(NodeIdResponse { node_id })
        .map(|reply| reply::json(&reply))
        .map(|reply| reply::with_header(reply, http::header::CACHE_CONTROL, "no-cache"))
        .map_err(reject)
}
