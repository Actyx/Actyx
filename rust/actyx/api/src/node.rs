use std::collections::BTreeSet;

use actyx_sdk::{service::NodeInfoResponse, AppId, NodeId};
use actyx_util::version::NodeVersion;
use chrono::Utc;
use swarm::BanyanStore;
use warp::*;

use crate::{
    boxed_on_debug, or,
    util::{
        filters::{accept_text, authenticate, header_or_query_token},
        reject, Result,
    },
    NodeInfo,
};

fn with_node_id(node_id: NodeId) -> impl Filter<Extract = (NodeId,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || node_id)
}

pub fn with_store(
    store: BanyanStore,
) -> impl Filter<Extract = (BanyanStore,), Error = std::convert::Infallible> + Clone {
    any().map(move || store.clone())
}

pub fn with_node_info(info: NodeInfo) -> impl Filter<Extract = (NodeInfo,), Error = std::convert::Infallible> + Clone {
    any().map(move || info.clone())
}

pub(crate) fn route(
    node_info: NodeInfo,
    store: BanyanStore,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    or!(filter_id(node_info.clone()), filter_info(node_info, store))
}

fn filter_id(node_info: NodeInfo) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let node_id = node_info.node_id;
    path("id")
        .and(path::end())
        .and(accept_text())
        .and(with_node_id(node_id))
        .and_then(handle_id)
}

async fn handle_id(node_id: NodeId) -> Result<impl Reply> {
    Ok(node_id.to_string())
        .map(|reply| reply::with_header(reply, http::header::CACHE_CONTROL, "no-cache"))
        .map_err(reject)
}

fn filter_info(
    node_info: NodeInfo,
    store: BanyanStore,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    path("info")
        .and(path::end())
        .and(get())
        .and(authenticate(node_info.clone(), header_or_query_token()))
        .and(with_store(store))
        .and(with_node_info(node_info))
        .and_then(handle_info)
}

async fn handle_info(_app_id: AppId, store: BanyanStore, node_info: NodeInfo) -> Result<impl Reply> {
    let connected_nodes = store
        .ipfs()
        .connections()
        .into_iter()
        .map(|(p, _)| p)
        .collect::<BTreeSet<_>>()
        .len();

    Utc::now()
        .signed_duration_since(node_info.started_at)
        .to_std()
        .map_err(|_| anyhow::anyhow!("Time on the node went backwards"))
        .map(|uptime| NodeInfoResponse {
            connected_nodes,
            uptime,
            version: NodeVersion::get().to_string(),
        })
        .map(|r| reply::json(&r))
        .map_err(reject)
}
