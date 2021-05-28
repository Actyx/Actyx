mod auth;
mod events;
mod ipfs_file_gateway;
mod node_id;
mod rejections;
#[cfg(test)]
mod tests;
mod util;

use actyx_util::{ax_panic, formats::NodeErrorContext};
use futures::future::try_join_all;
use std::net::SocketAddr;
use swarm::{event_store::EventStore, BanyanStore};
use warp::*;

use crate::util::hyper_serve::serve_it;
pub use crate::util::NodeInfo;
pub use crate::util::{AppMode, BearerToken, Token};

pub async fn run(node_info: NodeInfo, store: BanyanStore, bind_to: impl Iterator<Item = SocketAddr> + Send) {
    let api = routes(node_info, store);
    let tasks = bind_to
        .into_iter()
        .map(|i| {
            serve_it(i, api.clone().boxed()).map_err(move |e| {
                e.context(NodeErrorContext::BindFailed {
                    port: i.port(),
                    component: "API".into(),
                })
            })
        })
        .map(|i| async move {
            let (addr, task) = i?;
            tracing::info!(target: "API_BOUND", "API bound to {}.", addr);
            task.await
        })
        .collect::<Vec<_>>();
    // This error will be propagated by a `panic!`, so we use the `ax_panic!`
    // macro, which will wrap the error into an `Arc` in order to properly
    // extract it later in the node's panic hook
    if let Err(e) = try_join_all(tasks).await {
        ax_panic!(e);
    }
}

fn routes(node_info: NodeInfo, store: BanyanStore) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let event_store = EventStore::new(store.clone());
    let event_service = events::service::EventService::new(event_store);
    let events = events::routes(node_info.clone(), event_service);
    let node_id = node_id::route(node_info.clone());
    let auth = auth::route(node_info);

    let api_path = path!("api" / "v2" / ..);
    let cors = cors()
        .allow_any_origin()
        .allow_headers(vec!["accept", "authorization", "content-type"])
        .allow_methods(&[http::Method::GET, http::Method::POST]);

    path("ipfs")
        .and(ipfs_file_gateway::route(store))
        .or(api_path.and(path("events")).and(events))
        .or(api_path.and(path("node_id")).and(node_id))
        .or(api_path.and(path("authenticate")).and(auth))
        .recover(|r| async { rejections::handle_rejection(r) })
        .with(cors)
}
