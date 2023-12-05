mod ans;
mod auth;
mod bearer_token;
mod blob;
mod events;
mod files;
mod filters;
mod hyper_serve;
pub mod licensing;
pub(crate) mod macros;
mod node;
mod rejections;
#[cfg(test)]
mod tests;

pub use crate::api::events::service::EventService;
use crate::{
    api::{files::FilePinner, hyper_serve::serve_it, licensing::Licensing},
    ax_panic, balanced_or,
    crypto::{KeyStoreRef, PublicKey},
    swarm::{blob_store::BlobStore, event_store_ref::EventStoreRef, BanyanStore},
    util::{
        formats::{NodeCycleCount, NodeErrorContext},
        to_multiaddr,
        variable::Reader,
        SocketAddrHelper,
    },
};
use ax_types::{service::SwarmState, NodeId};
use chrono::{DateTime, Utc};
use crossbeam::channel::Sender;
use derive_more::Display;
use futures::future::try_join_all;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};
use warp::{cors, path, Filter, Rejection, Reply};

#[derive(Clone)]
pub struct NodeInfo {
    pub node_id: NodeId,
    pub key_store: KeyStoreRef,
    pub token_validity: u32,
    pub cycles: NodeCycleCount,
    pub ax_public_key: PublicKey,
    pub licensing: Licensing,
    pub started_at: DateTime<Utc>,
}

impl NodeInfo {
    pub fn new(
        node_id: NodeId,
        key_store: KeyStoreRef,
        cycles: NodeCycleCount,
        licensing: Licensing,
        started_at: DateTime<Utc>,
    ) -> Self {
        Self {
            node_id,
            key_store,
            cycles,
            token_validity: 86400,
            ax_public_key: PublicKey::ax_public_key(),
            licensing,
            started_at,
        }
    }
}

#[derive(Debug, Display)]
pub struct Error(anyhow::Error); // anyhow::Error is sealed so we wrap it
impl std::error::Error for Error {}
impl warp::reject::Reject for Error {}

pub fn reject(err: anyhow::Error) -> Rejection {
    warp::reject::custom(Error(err))
}

pub type Result<T> = std::result::Result<T, Rejection>;

#[derive(Debug, Display, Deserialize)]
pub struct Token(pub(crate) String);

impl From<String> for Token {
    fn from(x: String) -> Self {
        Self(x)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AppMode {
    Trial,
    Signed,
}

pub async fn run(
    node_info: NodeInfo,
    store: BanyanStore,
    event_store: EventStoreRef,
    blobs: BlobStore,
    bind_to: Arc<Mutex<SocketAddrHelper>>,
    snd: Sender<anyhow::Result<()>>,
    swarm_state: Reader<SwarmState>,
) {
    let event_service = events::service::EventService::new(event_store, node_info.node_id);
    let pinner = FilePinner::new(event_service.clone(), store.ipfs().clone());
    let api = routes(node_info, store, event_service, pinner, blobs, swarm_state);
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
        .collect::<anyhow::Result<Vec<_>>>();
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
    blobs: BlobStore,
    swarm_state: Reader<SwarmState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let events = events::routes(node_info.clone(), event_service);
    let node = node::route(node_info.clone(), store.clone(), swarm_state);
    let auth = auth::route(node_info.clone());
    let files = files::route(store.clone(), node_info.clone(), pinner);
    let blob = blob::routes(blobs, node_info.clone());

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
