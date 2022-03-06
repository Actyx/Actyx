mod ans;
mod auth;
mod blob;
mod events;
mod files;
pub mod formats;
mod node;
mod rejections;
#[cfg(test)]
mod tests;
mod util;

use actyx_util::{ax_panic, formats::NodeErrorContext};
use anyhow::Result;
use crossbeam::channel::Sender;
use futures::future::try_join_all;
use std::fmt;
use swarm::{event_store_ref::EventStoreRef, BanyanStore};
use warp::*;

pub use crate::events::service::EventService;
pub use crate::util::{AppMode, BearerToken, NodeInfo, Token};
use crate::{files::FilePinner, util::hyper_serve::serve_it};
use actyx_util::{to_multiaddr, SocketAddrHelper};
use parking_lot::Mutex;
use std::sync::Arc;

pub async fn run(
    node_info: NodeInfo,
    store: BanyanStore,
    event_store: EventStoreRef,
    bind_to: Arc<Mutex<SocketAddrHelper>>,
    snd: Sender<anyhow::Result<()>>,
) {
    let event_service = events::service::EventService::new(event_store, node_info.node_id);
    let pinner = FilePinner::new(event_service.clone(), store.ipfs().clone());
    let api = routes(node_info, store, event_service, pinner);
    #[allow(clippy::needless_collect)]
    // following clippy here would lead to deadlock, d’oh
    let addrs = bind_to.lock().iter().collect::<Vec<_>>();
    let tasks = addrs
        .into_iter()
        .map(|i| {
            let (addr, task) = serve_it(i, api.clone().boxed()).map_err(move |e| {
                e.context(NodeErrorContext::BindFailed {
                    addr: to_multiaddr(i),
                    component: "API".into(),
                })
            })?;
            tracing::info!(target: "API_BOUND", "API bound to {}.", addr);
            bind_to.lock().inject_bound_addr(i, addr);
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
    event_service: EventService,
    pinner: FilePinner,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let events = events::routes(node_info.clone(), event_service);
    let node = node::route(node_info.clone(), store.clone());
    let auth = auth::route(node_info.clone());
    let files = files::route(store.clone(), node_info.clone(), pinner);
    let blob = blob::routes(store.clone(), node_info.clone());

    let api_path = path!("api" / "v2" / ..);
    let cors = cors()
        .allow_any_origin()
        .allow_headers(vec!["accept", "authorization", "content-type"])
        .allow_methods(&[http::Method::GET, http::Method::POST, http::Method::PUT]);

    let log = warp::log::custom(|info| {
        tracing::debug!(
            remote_addr=%OptFmt(info.remote_addr()),
            method=%info.method(),
            path=%info.path(),
            version=?info.version(),
            status=%info.status().as_u16(),
            referer=%OptFmt(info.referer()),
            user_agent=%OptFmt(info.user_agent()),
            elapsed=?info.elapsed(),
            "Processed request"
        );
    });
    balanced_or!(
        files::root_serve(store, node_info),
        api_path.and(balanced_or!(
            path("events").and(events),
            path("node").and(node),
            path("auth").and(auth),
            path("files").and(files),
            path("blob").and(blob),
        ))
    )
    .recover(|r| async { rejections::handle_rejection(r) })
    .with(cors)
    .with(log)
}

struct OptFmt<T>(Option<T>);

impl<T: fmt::Display> fmt::Display for OptFmt<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref t) = self.0 {
            fmt::Display::fmt(t, f)
        } else {
            f.write_str("-")
        }
    }
}
