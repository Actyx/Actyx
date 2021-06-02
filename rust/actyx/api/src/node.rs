use actyxos_sdk::NodeId;
use warp::*;

use crate::{
    util::{filters::accept_text, reject, Result},
    NodeInfo,
};

fn with_node_id(node_id: NodeId) -> impl Filter<Extract = (NodeId,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || node_id)
}

pub(crate) fn route(node_info: NodeInfo) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let node_id = node_info.node_id;
    get().and(accept_text()).and(with_node_id(node_id)).and_then(handle)
}

async fn handle(node_id: NodeId) -> Result<impl Reply> {
    Ok(node_id.to_string())
        .map(|reply| reply::with_header(reply, http::header::CACHE_CONTROL, "no-cache"))
        .map_err(reject)
}
