mod ans;
mod auth;
mod events;
pub mod formats;
mod ipfs_file_gateway;
mod node;
mod rejections;
#[cfg(test)]
mod tests;
mod util;

use actyx_util::{ax_panic, formats::NodeErrorContext};
use anyhow::Result;
use crossbeam::channel::Sender;
use futures::future::try_join_all;
use std::net::SocketAddr;
use swarm::{event_store_ref::EventStoreRef, BanyanStore};
use warp::*;

pub use crate::events::service::EventService;
use crate::util::hyper_serve::serve_it;
pub use crate::util::{AppMode, BearerToken, NodeInfo, Token};

pub async fn run(
    node_info: NodeInfo,
    store: BanyanStore,
    event_store: EventStoreRef,
    bind_to: impl Iterator<Item = SocketAddr> + Send,
    snd: Sender<anyhow::Result<()>>,
) {
    let api = routes(node_info, store, event_store);
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
        .map(|i| {
            let (addr, task) = i?;
            tracing::info!(target: "API_BOUND", "API bound to {}.", addr);
            Ok(task)
        })
        .collect::<Result<Vec<_>>>();
    let tasks = match tasks {
        Ok(t) => t,
        Err(e) => {
            ax_panic!(e);
        }
    };

    // now we know that binding was successful
    let _ = snd.send(Ok(()));

    // This error will be propagated by a `panic!`, so we use the `ax_panic!`
    // macro, which will wrap the error into an `Arc` in order to properly
    // extract it later in the node's panic hook
    if let Err(e) = try_join_all(tasks).await {
        ax_panic!(e);
    }
}

fn routes(
    node_info: NodeInfo,
    store: BanyanStore,
    event_store: EventStoreRef,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let event_service = events::service::EventService::new(event_store, node_info.node_id);
    let events = events::routes(node_info.clone(), event_service);
    let node = node::route(node_info.clone(), store.clone());
    let auth = auth::route(node_info.clone());

    let files_route = ipfs_file_gateway::files_route(store.clone(), node_info);

    let api_path = path!("api" / "v2" / ..);
    let cors = cors()
        .allow_any_origin()
        .allow_headers(vec!["accept", "authorization", "content-type"])
        .allow_methods(&[http::Method::GET, http::Method::POST]);

    api_path
        .and(path("events"))
        .and(events)
        .or(api_path.and(path("node")).and(node))
        .or(api_path.and(path("auth")).and(auth))
        .or(files_route)
        .or(path("ipfs").and(ipfs_file_gateway::route(store)))
        .recover(|r| async { rejections::handle_rejection(r) })
        .with(cors)

}
