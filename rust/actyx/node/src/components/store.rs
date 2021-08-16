use super::{Component, ComponentRequest};
use crate::{node_settings::Settings, BindTo};
use actyx_sdk::NodeId;
use anyhow::Result;
use api::formats::Licensing;
use api::NodeInfo;
use chrono::{DateTime, Utc};
use crossbeam::channel::{Receiver, Sender};
use crypto::KeyStoreRef;
use parking_lot::Mutex;
use std::{convert::TryInto, path::PathBuf, sync::Arc};
use swarm::{
    event_store_ref::{EventStoreHandler, EventStoreRef, EventStoreRequest},
    BanyanStore, SwarmConfig,
};
use tokio::sync::oneshot;
use tracing::*;
use util::formats::{Connection, NodeCycleCount, Peer};

pub(crate) enum StoreRequest {
    NodesInspect(oneshot::Sender<Result<InspectResponse>>),
    EventsV2(EventStoreRequest),
}

pub(crate) struct InspectResponse {
    pub peer_id: String,
    pub swarm_addrs: Vec<String>,
    pub announce_addrs: Vec<String>,
    pub connections: Vec<Connection>,
    pub known_peers: Vec<Peer>,
}

pub(crate) type StoreTx = Sender<ComponentRequest<StoreRequest>>;

// Dynamic config
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StoreConfig {
    swarm_config: SwarmConfig,
    licensing: Licensing,
}

impl Component<StoreRequest, StoreConfig> for Store {
    fn get_type() -> &'static str {
        "Swarm"
    }
    fn get_rx(&self) -> &Receiver<ComponentRequest<StoreRequest>> {
        &self.rx
    }
    fn handle_request(&mut self, req: StoreRequest) -> Result<()> {
        match req {
            StoreRequest::NodesInspect(tx) => {
                if let Some(InternalStoreState { store, .. }) = self.state.as_ref() {
                    let peer_id = store.ipfs().local_peer_id().to_string();
                    let swarm_addrs: Vec<_> = store
                        .ipfs()
                        .listeners()
                        .into_iter()
                        .map(|addr| addr.to_string())
                        .collect();
                    let announce_addrs: Vec<_> = store
                        .ipfs()
                        .external_addresses()
                        .into_iter()
                        .map(|rec| rec.addr.to_string())
                        .collect();
                    let connections: Vec<_> = store
                        .ipfs()
                        .connections()
                        .into_iter()
                        .map(|(peer, addr)| Connection {
                            peer_id: peer.to_string(),
                            addr: addr.to_string(),
                        })
                        .collect();
                    let known_peers: Vec<_> = store
                        .ipfs()
                        .peers()
                        .into_iter()
                        .filter_map(|peer| {
                            let info = store.ipfs().peer_info(&peer)?;
                            Some(Peer {
                                peer_id: peer.to_string(),
                                addrs: info.addresses().map(|(addr, _)| addr.to_string()).collect(),
                            })
                        })
                        .collect();
                    let _ = tx.send(Ok(InspectResponse {
                        peer_id,
                        swarm_addrs,
                        announce_addrs,
                        connections,
                        known_peers,
                    }));
                } else {
                    let _ = tx.send(Err(anyhow::anyhow!("Store not running")));
                }
            }
            StoreRequest::EventsV2(request) => {
                if let Some(InternalStoreState { rt, events, .. }) = self.state.as_mut() {
                    events.handle(request, rt.handle());
                }
            }
        }
        Ok(())
    }
    fn set_up(&mut self, settings: StoreConfig) -> bool {
        self.store_config = Some(settings);
        true
    }
    fn start(&mut self, snd: Sender<anyhow::Result<()>>) -> Result<()> {
        debug_assert!(self.state.is_none());
        if let Some(cfg) = self.store_config.clone() {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(self.number_of_threads.unwrap_or(2))
                .enable_all()
                .build()?;
            let bind_to = self.bind_to.clone();
            let node_info = NodeInfo::new(
                self.node_id,
                self.keystore.clone(),
                self.node_cycle_count,
                cfg.licensing.clone(),
                self.started_at,
            );
            // client creation is setting up some tokio timers and therefore
            // needs to be called with a tokio runtime
            let event_store = self.event_store.clone();
            let store = rt.block_on(async move {
                let store = BanyanStore::new(cfg.swarm_config).await?;

                store.spawn_task(
                    "api",
                    api::run(node_info, store.clone(), event_store, bind_to.api.into_iter(), snd),
                );
                Ok::<BanyanStore, anyhow::Error>(store)
            })?;

            let events = EventStoreHandler::new(store.clone());
            self.state = Some(InternalStoreState { rt, store, events });
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
        let keypair = self
            .keystore
            .read()
            .get_pair(self.node_id.into())
            .ok_or_else(|| anyhow::anyhow!("No KeyPair available for KeyId {}", self.node_id))?;
        let psk: [u8; 32] = base64::decode(&s.swarm.swarm_key)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid psk"))?;
        let topic = s.swarm.topic.replace('/', "_");
        let db_path = self.working_dir.join(format!("{}.sqlite", topic));
        let read_only = s.api.events.read_only;
        let swarm_config = SwarmConfig {
            topic,
            index_store: Some(self.db.clone()),
            keypair: Some(keypair),
            psk: Some(psk),
            node_name: Some(s.admin.display_name),
            db_path: Some(db_path),
            external_addresses: s
                .swarm
                .announce_addresses
                .iter()
                .map(|s| s.parse())
                .collect::<Result<_, libp2p::multiaddr::Error>>()?,
            listen_addresses: self.bind_to.swarm.clone().to_multiaddrs().collect(),
            bootstrap_addresses: s
                .swarm
                .initial_peers
                .iter()
                .map(|s| s.parse())
                .collect::<Result<_, libp2p::multiaddr::Error>>()?,
            enable_fast_path: !read_only,
            enable_slow_path: !read_only,
            enable_root_map: !read_only,
            ..SwarmConfig::basic()
        };
        Ok(StoreConfig {
            swarm_config,
            licensing: s.licensing,
        })
    }
}
struct InternalStoreState {
    rt: tokio::runtime::Runtime,
    store: BanyanStore,
    events: EventStoreHandler,
}
/// Struct wrapping the store service and handling its lifecycle.
pub(crate) struct Store {
    rx: Receiver<ComponentRequest<StoreRequest>>,
    event_store: EventStoreRef,
    state: Option<InternalStoreState>,
    store_config: Option<StoreConfig>,
    working_dir: PathBuf,
    bind_to: BindTo,
    keystore: KeyStoreRef,
    node_id: NodeId,
    db: Arc<Mutex<rusqlite::Connection>>,
    number_of_threads: Option<usize>,
    node_cycle_count: NodeCycleCount,
    started_at: DateTime<Utc>,
}

impl Store {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rx: Receiver<ComponentRequest<StoreRequest>>,
        event_store: EventStoreRef,
        working_dir: PathBuf,
        bind_to: BindTo,
        keystore: KeyStoreRef,
        node_id: NodeId,
        db: Arc<Mutex<rusqlite::Connection>>,
        node_cycle_count: NodeCycleCount,
    ) -> anyhow::Result<Self> {
        std::fs::create_dir_all(working_dir.clone())?;
        Ok(Self {
            rx,
            event_store,
            state: None,
            store_config: None,
            working_dir,
            bind_to,
            keystore,
            node_id,
            db,
            number_of_threads: None,
            node_cycle_count,
            started_at: Utc::now(),
        })
    }
}
