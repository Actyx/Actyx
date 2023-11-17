mod ans;
mod auth;
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

use crate::{
    api::{files::FilePinner, hyper_serve::serve_it},
    ax_panic, balanced_or,
    swarm::{blob_store::BlobStore, event_store_ref::EventStoreRef, BanyanStore},
    util::{formats::NodeErrorContext, to_multiaddr, variable::Reader, SocketAddrHelper},
};
use actyx_sdk::service::SwarmState;
use crossbeam::channel::Sender;
use futures::future::try_join_all;
use parking_lot::Mutex;
use std::{fmt, sync::Arc};
use warp::{cors, path, Filter, Rejection, Reply};

pub use crate::api::events::service::EventService;

use std::{str::FromStr, time::Duration};

use crate::{
    api::licensing::Licensing,
    crypto::{KeyStoreRef, PublicKey},
    util::formats::NodeCycleCount,
};
use actyx_sdk::{AppId, NodeId, Timestamp};
use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};

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
            token_validity: get_token_validity(),
            ax_public_key: get_ax_public_key(),
            licensing,
            started_at,
        }
    }
}

pub(crate) fn get_ax_public_key() -> PublicKey {
    PublicKey::from_str(option_env!("AX_PUBLIC_KEY").unwrap_or("075i62XGQJuXjv6nnLQyJzECZhF29acYvYeEOJ3kc5M8="))
        .unwrap()
}

fn get_token_validity() -> u32 {
    86400
}

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

#[derive(Debug, Clone, Deserialize, Serialize, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BearerToken {
    /// when it was created
    pub created: Timestamp,
    /// for whom
    pub app_id: AppId,
    /// restart cycle count of Actyx node that created it
    pub cycles: NodeCycleCount,
    /// app version
    pub app_version: String,
    /// intended validity in seconds
    pub validity: u32,
    /// App mode,
    pub app_mode: AppMode,
}

impl BearerToken {
    pub fn is_expired(&self) -> bool {
        Timestamp::now() > self.expiration()
    }

    pub fn expiration(&self) -> Timestamp {
        self.created + Duration::from_secs(self.validity.into())
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

#[cfg(test)]
mod bearer_token_tests {
    use actyx_sdk::{app_id, Timestamp};
    use std::time::Duration;

    use super::{AppMode, BearerToken};

    #[test]
    fn bearer_token_is_expired() {
        let token = BearerToken {
            created: Timestamp::now() - Duration::from_secs(2),
            app_id: app_id!("app-id"),
            cycles: 0.into(),
            app_version: "1.0.0".into(),
            validity: 1,
            app_mode: AppMode::Signed,
        };
        assert!(token.is_expired());

        let token = BearerToken {
            created: Timestamp::now(),
            app_id: app_id!("app-id"),
            cycles: 0.into(),
            app_version: "1.0.0".into(),
            validity: 300,
            app_mode: AppMode::Signed,
        };
        assert!(!token.is_expired());
    }

    #[test]
    fn bearer_token_expiration() {
        let now = Timestamp::now();
        let token = BearerToken {
            created: now,
            app_id: app_id!("app-id"),
            cycles: 0.into(),
            app_version: "1.0.0".into(),
            validity: 1,
            app_mode: AppMode::Signed,
        };
        assert_eq!(token.expiration(), now + Duration::from_secs(token.validity as u64));
    }

    #[test]
    fn bearer_round_trip() {
        let token = BearerToken {
            created: Timestamp::now(),
            app_id: app_id!("app-id"),
            cycles: 0.into(),
            app_version: "1.0.0".into(),
            validity: 1,
            app_mode: AppMode::Signed,
        };
        let json = serde_json::to_string(&token).unwrap();
        let round_tripped = serde_json::from_str(&json).unwrap();
        assert_eq!(token, round_tripped);
    }

    #[test]
    fn bearer_wire_format() {
        let json = r#"{
              "created": 1619769229417484,
              "appId": "app-id",
              "cycles": 42,
              "appVersion": "1.4.2",
              "validity": 10,
              "appMode": "signed"
            }"#;
        let des: BearerToken = serde_json::from_str(json).unwrap();
        let token = BearerToken {
            created: Timestamp::from(1619769229417484),
            app_id: app_id!("app-id"),
            cycles: 42.into(),
            app_version: "1.4.2".into(),
            validity: 10,
            app_mode: AppMode::Signed,
        };
        assert_eq!(des, token);
    }
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
