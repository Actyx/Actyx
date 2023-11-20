use super::{Component, ComponentRequest};
use crate::{
    api::{licensing::Licensing, NodeInfo},
    crypto::KeyStoreRef,
    node::{node_settings::Settings, BindTo},
    swarm::{
        blob_store::BlobStore,
        event_store_ref::{EventStoreHandler, EventStoreRef, EventStoreRequest},
        BanyanStore, DbPath, EphemeralEventsConfig, EventRoute, GossipMessage, Ipfs, SwarmConfig,
    },
    util::{
        formats::{Connection, Failure, NodeCycleCount, Peer, PeerInfo, PingStats},
        variable::Reader,
        SocketAddrHelper,
    },
};
use acto::ActoRef;
use anyhow::Result;
use ax_sdk::{service::SwarmState, NodeId};
use chrono::{DateTime, SecondsFormat::Millis, Utc};
use crossbeam::channel::{Receiver, Sender};
use futures::FutureExt;
use ipfs_embed::{Direction, PeerId};
use libp2p::{multiaddr::Protocol, Multiaddr};
use parking_lot::Mutex;
use std::{
    convert::TryInto,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::oneshot;
use tracing::*;

pub(crate) enum StoreRequest {
    NodesInspect(oneshot::Sender<Result<InspectResponse>>),
    EventsV2(EventStoreRequest),
    ActiveTopic(oneshot::Sender<String>),
}

impl std::fmt::Debug for StoreRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodesInspect(_) => f.debug_tuple("NodesInspect").finish(),
            Self::EventsV2(arg0) => {
                let req = arg0.to_string();
                f.debug_tuple("EventsV2").field(&req.as_str()).finish()
            }
            Self::ActiveTopic(_) => f.debug_tuple("ActiveTopic").finish(),
        }
    }
}

#[derive(Debug)]
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

fn without_peer(addr: &Multiaddr) -> String {
    if matches!(addr.iter().last(), Some(Protocol::P2p(_))) {
        let mut addr = addr.clone();
        addr.pop();
        addr.to_string()
    } else {
        addr.to_string()
    }
}

fn swarm_addrs(ipfs: &Ipfs) -> Vec<String> {
    ipfs.listeners().into_iter().map(|addr| addr.to_string()).collect()
}

fn announce_addrs(ipfs: &Ipfs) -> Vec<String> {
    ipfs.external_addresses()
        .into_iter()
        .map(|rec| rec.addr.to_string())
        .collect()
}

fn connections(ipfs: &Ipfs) -> Vec<Connection> {
    ipfs.connections()
        .into_iter()
        .map(|(peer, addr, dt, dir)| Connection {
            peer_id: peer.to_string(),
            addr: without_peer(&addr),
            since: dt.to_rfc3339_opts(Millis, true),
            outbound: dir == Direction::Outbound,
        })
        .collect()
}

fn known_peers(ipfs: &Ipfs) -> Vec<Peer> {
    ipfs.peers()
        .into_iter()
        .filter_map(|peer| {
            let info = ipfs.peer_info(&peer)?;
            let mut addrs = Vec::new();
            let mut addr_source = Vec::new();
            let mut addr_since = Vec::new();
            for (addr, s, dt) in info.addresses() {
                addrs.push(without_peer(addr));
                addr_source.push(format!("{:?}", s));
                addr_since.push(dt.to_rfc3339_opts(Millis, true));
            }
            let ping_stats = info.full_rtt().map(|rtt| PingStats {
                current: rtt.current().min(Duration::from_secs(3600)).as_micros() as u32,
                decay_3: rtt.decay_3().min(Duration::from_secs(3600)).as_micros() as u32,
                decay_10: rtt.decay_10().min(Duration::from_secs(3600)).as_micros() as u32,
                failures: rtt.failures(),
                failure_rate: rtt.failure_rate(),
            });
            let failures = info
                .recent_failures()
                .map(|f| Failure {
                    addr: f.addr().to_string(),
                    time: f.time().to_rfc3339_opts(Millis, true),
                    display: f.display().to_owned(),
                    details: f.debug().to_owned(),
                })
                .collect();
            let peer_info = PeerInfo {
                protocol_version: info.protocol_version().map(ToOwned::to_owned),
                agent_version: info.agent_version().map(ToOwned::to_owned),
                protocols: info.protocols().map(|s| s.to_owned()).collect(),
                listeners: info.listen_addresses().map(|a| a.to_string()).collect(),
            };
            Some(Peer {
                peer_id: peer.to_string(),
                info: peer_info,
                addrs,
                addr_source,
                addr_since,
                failures,
                ping_stats,
            })
        })
        .collect()
}

impl Component<StoreRequest, StoreConfig> for Store {
    fn get_type() -> &'static str {
        "Swarm"
    }
    fn get_rx(&self) -> &Receiver<ComponentRequest<StoreRequest>> {
        &self.rx
    }
    fn handle_request(&mut self, req: StoreRequest) -> Result<()> {
        tracing::debug!("handling request {:?}", req);
        match req {
            StoreRequest::NodesInspect(tx) => {
                if let Some(InternalStoreState { store, .. }) = self.state.as_ref() {
                    let peer_id = store.ipfs().local_peer_id().to_string();
                    let ipfs = store.ipfs();
                    let _ = tx.send(Ok(InspectResponse {
                        peer_id,
                        swarm_addrs: swarm_addrs(ipfs),
                        announce_addrs: announce_addrs(ipfs),
                        connections: connections(ipfs),
                        known_peers: known_peers(ipfs),
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
            StoreRequest::ActiveTopic(tx) => {
                let state = self.state.as_ref().expect("Internal store state should be valid.");
                let _ = tx.send(state.store.get_topic());
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
                .thread_name_fn(|| {
                    static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                    let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                    format!("store-worker-{}", id)
                })
                .worker_threads(self.number_of_threads.unwrap_or(2))
                .enable_all()
                .build()?;
            let bind_api = self.bind_api.clone();
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
            let swarm_config = cfg.swarm_config;
            let swarm_observer = self.swarm_observer.clone();
            let swarm_state = self.swarm_state.clone();
            let store = rt.block_on(async move {
                let blobs = BlobStore::new(
                    swarm_config
                        .blob_store
                        .clone()
                        .map(DbPath::File)
                        .unwrap_or(DbPath::Memory),
                )?;
                let store = BanyanStore::new(swarm_config, swarm_observer).await?;
                store.spawn_task(
                    "api".to_owned(),
                    crate::api::run(node_info, store.clone(), event_store, blobs, bind_api, snd, swarm_state).boxed(),
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
        let index_store = Some(self.working_dir.join(format!("{}-index", topic)));
        let blob_store = Some(self.working_dir.join(format!("{}-blobs", topic)));
        let read_only = s.api.events.read_only;

        let event_routes = s
            .event_routing
            .routes
            .into_iter()
            .map(|e| EventRoute::new(e.from, e.into))
            .collect();
        let ephemeral_event_config = EphemeralEventsConfig::from(s.event_routing.streams);

        let swarm_config = SwarmConfig {
            topic,
            index_store,
            blob_store,
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
            listen_addresses: self.bind_swarm.clone(),
            bootstrap_addresses: s
                .swarm
                .initial_peers
                .iter()
                .map(|s| s.parse())
                .collect::<Result<_, libp2p::multiaddr::Error>>()?,
            enable_fast_path: !read_only,
            enable_slow_path: !read_only,
            enable_root_map: !read_only,
            enable_mdns: s.swarm.mdns,
            block_cache_count: s.swarm.block_cache_count,
            block_cache_size: s.swarm.block_cache_size,
            block_gc_interval: Duration::from_secs(s.swarm.block_gc_interval),
            enable_metrics: s.swarm.metrics_interval > 0,
            metrics_interval: Duration::from_secs(s.swarm.metrics_interval),
            ping_timeout: Duration::from_secs(s.swarm.ping_timeout),
            bitswap_timeout: Duration::from_secs(s.swarm.bitswap_timeout),
            branch_cache_size: s.swarm.branch_cache_size,
            cadence_root_map: Duration::from_secs(s.swarm.gossip_interval),
            event_routes,
            ephemeral_event_config,
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
    bind_swarm: Arc<Mutex<SocketAddrHelper>>,
    bind_api: Arc<Mutex<SocketAddrHelper>>,
    keystore: KeyStoreRef,
    node_id: NodeId,
    number_of_threads: Option<usize>,
    node_cycle_count: NodeCycleCount,
    started_at: DateTime<Utc>,
    swarm_observer: ActoRef<(PeerId, GossipMessage)>,
    swarm_state: Reader<SwarmState>,
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
        node_cycle_count: NodeCycleCount,
        swarm_observer: ActoRef<(PeerId, GossipMessage)>,
        swarm_state: Reader<SwarmState>,
    ) -> anyhow::Result<Self> {
        std::fs::create_dir_all(working_dir.clone())?;
        Ok(Self {
            rx,
            event_store,
            state: None,
            store_config: None,
            working_dir,
            bind_swarm: Arc::new(Mutex::new(bind_to.swarm)),
            bind_api: Arc::new(Mutex::new(bind_to.api)),
            keystore,
            node_id,
            number_of_threads: None,
            node_cycle_count,
            started_at: Utc::now(),
            swarm_observer,
            swarm_state,
        })
    }
}
