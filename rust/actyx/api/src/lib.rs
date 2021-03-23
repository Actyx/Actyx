mod event_service_api;
mod ipfs_file_gateway;
mod rejections;
#[cfg(test)]
mod tests;
mod util;

use std::{net::SocketAddr, sync::Arc};

use actyx_util::{ax_panic, formats::NodeErrorContext};
use actyxos_sdk::NodeId;
use crypto::KeyStoreRef;
use futures::{future::try_join_all, TryFutureExt};
use swarm::BanyanStore;
use warp::*;

use crate::util::hyper_serve::serve_it;

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
    // This error will be propagated by a `panic!`, so we have to wrap this into
    // an `Arc` in order to properly extract it later in the node's panic hook
    if let Err(e) = try_join_all(tasks).map_err(Arc::new).await {
        ax_panic!(e);
    }
}

fn routes(
    node_id: NodeId,
    store: BanyanStore,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let event_service = event_service_api::service::EventService::new(store.clone());
    let event_service_api = event_service_api::routes(node_id, event_service, key_store);

    let ipfs_file_gw = ipfs_file_gateway::route(store);

    let cors = cors()
        .allow_any_origin()
        .allow_headers(vec!["Content-Type", "content-type"])
        .allow_methods(&[http::Method::GET, http::Method::POST]);

    let crash = path!("_crash").and_then(|| async move { Err::<String, _>(reject::custom(rejections::Crash)) });

    crash
        .or(path("ipfs").and(ipfs_file_gw))
        // Note: event_service_api has a explicit rejection handler, which also
        // returns 404 no route matched. Thus it needs to come last. This should
        // eventually be refactored as part of Event Service v2.
        .or(path("api").and(path("v2").and(path("events")).and(event_service_api)))
        .recover(|r| async { rejections::handle_rejection(r) })
        .with(cors)
}
