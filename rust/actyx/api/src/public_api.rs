use futures::{future::try_join_all, TryFutureExt};
use std::{net::SocketAddr, sync::Arc};
use util::{ax_panic, formats::node_error_context};
use warp::{http::Method, Filter};

use crate::hyper_serve::serve_it;
use crate::{event_service_api, ipfs_file_gateway::create_gateway_route};
use crypto::KeyStoreRef;
use swarm::BanyanStore;

pub async fn run(store: BanyanStore, bind_to: impl Iterator<Item = SocketAddr> + Send, key_store: KeyStoreRef) {
    let event_service = event_service_api::service::EventService::new(store.clone());
    let event_service_api = warp::path("api").and(
        warp::path("v2")
            .and(warp::path("events"))
            .and(event_service_api::routes(event_service, key_store)),
    );
    let ipfs_file_gw = create_gateway_route(store.clone());

    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["Content-Type", "content-type"])
        .allow_methods(&[Method::GET, Method::POST]);

    let api = warp::path("ipfs")
        .and(ipfs_file_gw)
        // Note: event_service_api has a explicit rejection handler, which also
        // returns 404 no route matched. Thus it needs to come last. This should
        // eventually be refactored as part of Event Service v2.
        .or(event_service_api)
        .with(cors);

    let tasks = bind_to
        .into_iter()
        .map(|i| {
            serve_it(i, api.clone().boxed()).map_err(move |e| {
                e.context(node_error_context::Component("api".into()))
                    .context(node_error_context::BindingFailed(i.port()))
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
