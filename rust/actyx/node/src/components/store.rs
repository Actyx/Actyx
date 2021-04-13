use super::{Component, ComponentRequest};
use crate::{node_settings::Settings, BindTo};
use actyxos_sdk::NodeId;
use anyhow::Result;
use ax_config::StoreConfig;
use crossbeam::channel::{Receiver, Sender};
use crypto::KeyStoreRef;
use parking_lot::Mutex;
use std::{path::PathBuf, sync::Arc};
use swarm::{BanyanStore, NodeIdentity};
use tokio::sync::oneshot;
use tracing::*;

pub(crate) enum StoreRequest {
    GetSwarmState {
        tx: oneshot::Sender<Result<serde_json::Value>>,
    },
}
pub(crate) type StoreTx = Sender<ComponentRequest<StoreRequest>>;

impl Component<StoreRequest, StoreConfig> for Store {
    fn get_type(&self) -> &'static str {
        "Swarm"
    }
    fn get_rx(&self) -> &Receiver<ComponentRequest<StoreRequest>> {
        &self.rx
    }
    fn handle_request(&mut self, req: StoreRequest) -> Result<()> {
        match req {
            StoreRequest::GetSwarmState { tx } => {
                if let Some(InternalStoreState { rt: _, store }) = self.state.as_ref() {
                    let peer_id = store.ipfs().local_peer_id().to_string();
                    let listen_addrs: Vec<_> = store
                        .ipfs()
                        .listeners()
                        .into_iter()
                        .map(|addr| addr.to_string())
                        .collect();
                    let external_addrs: Vec<_> = store
                        .ipfs()
                        .external_addresses()
                        .into_iter()
                        .map(|rec| rec.addr.to_string())
                        .collect();
                    let peers: Vec<_> = store
                        .ipfs()
                        .connections()
                        .into_iter()
                        .map(|(peer, addr)| (peer.to_string(), addr.to_string()))
                        .collect();
                    let _ = tx.send(Ok(serde_json::json!({
                        "peer_id": peer_id,
                        "listen_addrs": listen_addrs,
                        "external_addrs": external_addrs,
                        "peers": peers,
                    })));
                } else {
                    let _ = tx.send(Err(anyhow::anyhow!("Store not running")));
                }
            }
        }
        Ok(())
    }
    fn set_up(&mut self, settings: StoreConfig) -> bool {
        self.store_config = Some(settings);
        true
    }
    fn start(&mut self, _err_notifier: Sender<anyhow::Error>) -> Result<()> {
        debug_assert!(self.state.is_none());
        if let Some(cfg) = self.store_config.as_ref() {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(cfg.number_of_threads.unwrap_or(2))
                .enable_all()
                .build()?;
            // client creation is setting up some tokio timers and therefore
            // needs to be called with a tokio runtime
            let keystore = self.keystore.clone();
            let db = self.db.clone();
            let store = rt.block_on(async move {
                let store = BanyanStore::from_axconfig_with_db(cfg.clone(), db).await?;
                store.spawn_task(
                    "api",
                    api::run(
                        store.node_id(),
                        store.clone(),
                        cfg.api_addr.clone().into_iter(),
                        keystore,
                    ),
                );
                Ok::<BanyanStore, anyhow::Error>(store)
            })?;

            self.state = Some(InternalStoreState { rt, store });
            Ok(())
        } else {
            anyhow::bail!("no config")
        }
    }
    fn stop(&mut self) -> Result<()> {
        if let Some(InternalStoreState { rt, .. }) = self.state.take() {
            debug!("Stopping the store");
            drop(rt);
        }
        Ok(())
    }
    fn extract_settings(&self, s: Settings) -> Result<StoreConfig> {
        let identity: NodeIdentity = self
            .keystore
            .read()
            .get_pair(self.node_id.into())
            .ok_or_else(|| anyhow::anyhow!("No KeyPair available for KeyId {}", self.node_id))?
            .into();
        let mut c = s.store_config(&self.working_dir)?;
        c.api_addr = self.bind_to.api.clone();
        c.ipfs_node.identity = Some(identity.to_string());
        c.ipfs_node.listen = self.bind_to.swarm.clone().to_multiaddrs().collect();
        Ok(c)
    }
}
struct InternalStoreState {
    rt: tokio::runtime::Runtime,
    store: BanyanStore,
}
/// Struct wrapping the store service and handling its lifecycle.
pub(crate) struct Store {
    rx: Receiver<ComponentRequest<StoreRequest>>,
    working_dir: PathBuf,
    bind_to: BindTo,
    keystore: KeyStoreRef,
    node_id: NodeId,
    store_config: Option<StoreConfig>,
    state: Option<InternalStoreState>,
    db: Arc<Mutex<rusqlite::Connection>>,
}

impl Store {
    pub fn new(
        rx: Receiver<ComponentRequest<StoreRequest>>,
        working_dir: PathBuf,
        bind_to: BindTo,
        keystore: KeyStoreRef,
        node_id: NodeId,
        db: Arc<Mutex<rusqlite::Connection>>,
    ) -> anyhow::Result<Self> {
        std::fs::create_dir_all(working_dir.clone())?;
        Ok(Self {
            state: None,
            rx,
            working_dir,
            bind_to,
            keystore,
            node_id,
            store_config: None,
            db,
        })
    }
}
