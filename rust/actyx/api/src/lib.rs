mod authentication_service_api;
mod event_service_api;
mod ipfs_file_gateway;
mod rejections;
#[cfg(test)]
mod tests;
mod util;

use std::net::SocketAddr;

use actyx_util::{ax_panic, formats::NodeErrorContext};
use actyxos_sdk::NodeId;
use crypto::KeyStoreRef;
use futures::future::try_join_all;
use swarm::BanyanStore;
use warp::*;

use crate::util::hyper_serve::serve_it;
pub use crate::util::{AppMode, BearerToken, Token};

pub async fn run(
    node_id: NodeId,
    store: BanyanStore,
    bind_to: impl Iterator<Item = SocketAddr> + Send,
    key_store: KeyStoreRef,
) {
    let api = routes(node_id, store, key_store);
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

fn routes(
    node_id: NodeId,
    store: BanyanStore,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let token_validity: u32 = if cfg!(debug_assertions) {
        std::env::var("AX_API_TOKEN_VALIDITY")
            .ok()
            .and_then(|x| x.parse().ok())
            .unwrap_or(86400) // 1 day
    } else {
        86400
    };

    let event_service = event_service_api::service::EventService::new(store.clone());

    let events = event_service_api::routes(node_id, event_service, key_store.clone());
    let auth = authentication_service_api::route(node_id.into(), key_store, token_validity);

    let api_path = path!("api" / "v2" / ..);
    let cors = cors()
        .allow_any_origin()
        .allow_headers(vec!["accept", "authorization", "content-type"])
        .allow_methods(&[http::Method::GET, http::Method::POST]);

    path("ipfs")
        .and(ipfs_file_gateway::route(store))
        .or(api_path.and(path("events")).and(events))
        .or(api_path.and(path("authenticate")).and(auth))
        .recover(|r| async { rejections::handle_rejection(r) })
        .with(cors)
}
