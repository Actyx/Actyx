//! Code structure
//!
//! ## BanyanStore
//! the externally visible interface
//! ## BanyanStoreData
//! the immutable and internally mutable part of the state - No logic implemented on this
//! ## BanyanStoreState
//! the mutable part of the state. No logic except drop implemented on this
//! ## BanyanStoreGuard
//! temporary struct that is created when acquiring mutable access to the state.
//! inside this you have mutable access to the state - but if you lock again you will deadlock.
pub mod access;
pub mod convert;
mod discovery;
mod gossip;
pub mod metrics;
mod prune;
mod sqlite;
mod sqlite_index_store;
mod streams;
pub mod transport;
mod unixfsv1;

#[cfg(test)]
mod tests;
mod v1;

pub use crate::sqlite_index_store::DbPath;
pub use crate::streams::StreamAlias;
pub use crate::v1::{EventStore, Present};

use crate::gossip::Gossip;
use crate::prune::RetainConfig;
use crate::sqlite::{SqliteStore, SqliteStoreWrite};
use crate::sqlite_index_store::SqliteIndexStore;
use crate::streams::{OwnStream, ReplicatedStreamInner};
use actyxos_sdk::{
    LamportTimestamp, NodeId, Offset, OffsetMap, OffsetOrMin, Payload, StreamId, StreamNr, TagSet, Timestamp,
};
use anyhow::{Context, Result};
use ax_futures_util::{prelude::*, stream::variable::Variable};
use banyan::{index::Index, query::Query, store::BranchCache, Config, FilteredChunk, Secrets, StreamBuilder};
use crypto::KeyPair;
use futures::{channel::mpsc, prelude::*};
use ipfs_embed::{
    BitswapConfig, Cid, Config as IpfsConfig, DnsConfig, ListenerEvent, Multiaddr, NetworkConfig, PeerId,
    StorageConfig, SyncEvent, ToLibp2p,
};
use libp2p::{
    dns::ResolverConfig,
    gossipsub::{GossipsubConfigBuilder, ValidationMode},
    identify::IdentifyConfig,
    multiaddr::Protocol,
    ping::PingConfig,
};
use maplit::btreemap;
use parking_lot::Mutex;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    convert::TryFrom,
    fmt::Debug,
    num::NonZeroU32,
    ops::{Deref, DerefMut, RangeInclusive},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use streams::*;
use trees::axtrees::{AxKey, AxTrees, Sha256Digest};
use util::{
    formats::NodeErrorContext,
    reentrant_safe_mutex::{ReentrantSafeMutex, ReentrantSafeMutexGuard},
};

#[allow(clippy::upper_case_acronyms)]
type TT = AxTrees;
type Key = AxKey;
type Event = Payload;
type Forest = banyan::Forest<TT, Event, SqliteStore>;
type Transaction = banyan::Transaction<TT, Event, SqliteStore, SqliteStoreWrite>;
type Tree = banyan::Tree<TT>;
type AxStreamBuilder = banyan::StreamBuilder<TT>;
type Link = Sha256Digest;

pub type Block = libipld::Block<libipld::DefaultParams>;
pub type Ipfs = ipfs_embed::Ipfs<libipld::DefaultParams>;

// TODO fix stream nr
static DISCOVERY_STREAM_NR: u64 = 1;
static METRICS_STREAM_NR: u64 = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct EphemeralEventsConfig {
    interval: Duration,
    streams: BTreeMap<StreamNr, RetainConfig>,
}
impl Default for EphemeralEventsConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30 * 60),
            streams: btreemap! {
                DISCOVERY_STREAM_NR.into() => RetainConfig::Events(1000),
                METRICS_STREAM_NR.into() => RetainConfig::Events(1000)
            },
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SwarmConfig {
    pub topic: String,
    pub index_store: Option<Arc<Mutex<rusqlite::Connection>>>,
    pub enable_mdns: bool,
    pub keypair: Option<KeyPair>,
    pub psk: Option<[u8; 32]>,
    pub node_name: Option<String>,
    pub db_path: Option<PathBuf>,
    pub external_addresses: Vec<Multiaddr>,
    pub listen_addresses: Vec<Multiaddr>,
    pub bootstrap_addresses: Vec<Multiaddr>,
    pub ephemeral_event_config: EphemeralEventsConfig,
    pub enable_fast_path: bool,
    pub enable_slow_path: bool,
    pub enable_root_map: bool,
    pub enable_discovery: bool,
    pub enable_metrics: bool,
}

impl SwarmConfig {
    pub fn test(node_name: &str) -> Self {
        Self {
            topic: "topic".into(),
            enable_mdns: false,
            node_name: Some(node_name.into()),
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".parse().unwrap()],
            enable_fast_path: true,
            enable_slow_path: true,
            enable_root_map: true,
            enable_discovery: true,
            enable_metrics: true,
            ..Default::default()
        }
    }
}

impl PartialEq for SwarmConfig {
    fn eq(&self, other: &Self) -> bool {
        self.topic == other.topic
            && self.enable_mdns == other.enable_mdns
            && self.keypair == other.keypair
            && self.psk == other.psk
            && self.node_name == other.node_name
            && self.db_path == other.db_path
            && self.external_addresses == other.external_addresses
            && self.listen_addresses == other.listen_addresses
            && self.bootstrap_addresses == other.bootstrap_addresses
            && self.ephemeral_event_config == other.ephemeral_event_config
            && self.enable_fast_path == other.enable_fast_path
            && self.enable_slow_path == other.enable_slow_path
            && self.enable_root_map == other.enable_root_map
            && self.enable_discovery == other.enable_discovery
            && self.enable_metrics == other.enable_metrics
    }
}

/// Stream manager.
#[derive(Clone)]
pub struct BanyanStore {
    data: Arc<BanyanStoreData>,
    state: Arc<ReentrantSafeMutex<BanyanStoreState>>,
}

#[derive(Clone, Debug, Default)]
struct SwarmOffsets {
    /// Currently validated OffsetMap
    present: OffsetMap,
    /// OffsetMap describing the replication target. Currently this is driven via `highest_seen`,
    /// but should eventually be fed by the partial replication mechanism.
    replication_target: OffsetMap,
}

/// All immutable or internally mutable parts of the banyan store
struct BanyanStoreData {
    gossip: Gossip,
    forest: Forest,
    ipfs: Ipfs,
    node_id: NodeId,
    /// maximum ingested offset and highest seen for each stream
    offsets: Variable<SwarmOffsets>,
    /// lamport timestamp for publishing to internal streams
    lamport: Variable<LamportTimestamp>,
}

/// Internal mutable state of the stream manager
///
/// Logic to manipulate the state is mostly implemented in BanyanStoreGuard
struct BanyanStoreState {
    /// the index store
    index_store: SqliteIndexStore,

    /// our own streams
    own_streams: BTreeMap<StreamNr, Arc<OwnStream>>,

    /// all remote nodes we know of
    remote_nodes: BTreeMap<NodeId, RemoteNodeInner>,

    /// dispatcher to tell interested parties of newly discovered streams
    known_streams: Vec<mpsc::UnboundedSender<StreamId>>,

    /// tasks of the stream manager.
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for BanyanStoreState {
    fn drop(&mut self) {
        for task in self.tasks.drain(..) {
            task.abort();
        }
    }
}

struct BanyanStoreGuard<'a> {
    /// the guard for the mutex - this implies that we have write access to the state
    guard: ReentrantSafeMutexGuard<'a, BanyanStoreState>,
    /// access to the immutable part of the store
    data: Arc<BanyanStoreData>,
    /// access to the state, here be dragons!
    state: Arc<ReentrantSafeMutex<BanyanStoreState>>,
}

impl<'a> Deref for BanyanStoreGuard<'a> {
    type Target = BanyanStoreState;
    fn deref(&self) -> &BanyanStoreState {
        self.guard.deref()
    }
}

impl<'a> DerefMut for BanyanStoreGuard<'a> {
    fn deref_mut(&mut self) -> &mut BanyanStoreState {
        self.guard.deref_mut()
    }
}

impl<'a> BanyanStoreGuard<'a> {
    fn outer(&self) -> BanyanStore {
        BanyanStore {
            data: self.data.clone(),
            state: self.state.clone(),
        }
    }

    fn node_id(&self) -> NodeId {
        self.data.node_id
    }

    fn local_stream_nrs(&self) -> Vec<StreamNr> {
        self.own_streams.keys().cloned().collect::<Vec<_>>()
    }

    fn local_stream_ids(&self) -> BTreeSet<StreamId> {
        self.own_streams.keys().map(|x| self.data.node_id.stream(*x)).collect()
    }

    fn received_lamport(&mut self, lamport: u64) -> anyhow::Result<u64> {
        self.index_store.received_lamport(lamport)
    }

    fn get_or_create_own_stream(&mut self, stream_nr: StreamNr) -> Arc<OwnStream> {
        if let Some(result) = self.own_streams.get(&stream_nr).cloned() {
            return result;
        }
        tracing::debug!("creating new own stream {}", stream_nr);
        let stream_id = self.node_id().stream(stream_nr);
        self.index_store
            .add_stream(stream_id)
            .expect("unable to write stream id");
        let stream = if let Some(root) = self
            .data
            .ipfs
            .resolve(&StreamAlias::from(stream_id))
            .expect("no alias for stream id")
        {
            let root = Link::try_from(root).expect("wrong link format");
            self.data
                .forest
                .load_stream_builder(Secrets::default(), Config::debug(), root)
                .expect("unable to load banyan tree")
        } else {
            StreamBuilder::new(Config::debug(), Secrets::default())
        };
        let stream = Arc::new(OwnStream::new(stream_nr, stream));
        self.own_streams.insert(stream_nr, stream.clone());
        tracing::debug!("publish new stream_id {}", stream_id);
        self.publish_new_stream_id(stream_id);
        stream
    }

    fn get_or_create_replicated_stream(&mut self, stream_id: StreamId) -> Arc<ReplicatedStreamInner> {
        debug_assert!(self.node_id() != stream_id.node_id());
        self.index_store
            .add_stream(stream_id)
            .expect("unable to write stream id");
        let node_id = stream_id.node_id();
        let stream_nr = stream_id.stream_nr();
        if let Some(stream) = self.get_or_create_remote_node(node_id).streams.get(&stream_nr).cloned() {
            return stream;
        }
        let tree = if let Some(root) = self.data.ipfs.resolve(&StreamAlias::from(stream_id)).unwrap() {
            let root = Link::try_from(root).expect("wrong link format");
            self.data
                .forest
                .load_tree(Secrets::default(), root)
                .expect("unable to load banyan tree")
        } else {
            Tree::default()
        };
        tracing::debug!("creating new replicated stream {}", stream_id);
        let stream = Arc::new(ReplicatedStreamInner::new(tree));
        self.get_or_create_remote_node(node_id)
            .streams
            .insert(stream_nr, stream.clone());
        let store = self.outer();
        self.spawn_task("careful_ingestion", store.careful_ingestion(stream_id, stream.clone()));
        tracing::debug!("publish new stream_id {}", stream_id);
        self.publish_new_stream_id(stream_id);
        stream
    }

    fn has_stream(&self, stream_id: StreamId) -> bool {
        let me = stream_id.node_id() == self.node_id();
        if me {
            self.own_streams.contains_key(&stream_id.stream_nr())
        } else {
            self.remote_nodes
                .get(&stream_id.node_id())
                .map(|node| node.streams.contains_key(&stream_id.stream_nr()))
                .unwrap_or_default()
        }
    }

    /// stream of latest updates from either gossip (for replicated streams) or internal updates
    ///
    /// note that this does not include event updates
    fn latest_stream(&mut self, stream_id: StreamId) -> impl Stream<Item = (LamportTimestamp, Offset)> {
        if stream_id.node_id() == self.node_id() {
            let stream = self.get_or_create_own_stream(stream_id.stream_nr());
            self.data
                .lamport
                .new_observer()
                .filter_map(move |lamport| {
                    future::ready(
                        Offset::from_offset_or_min(stream.snapshot().offset()).map(|offset| (lamport, offset)),
                    )
                })
                .left_stream()
        } else {
            self.get_or_create_replicated_stream(stream_id)
                .latest_seen()
                .new_observer()
                .filter_map(future::ready)
                .right_stream()
        }
    }

    /// Get a stream of trees for a given stream id
    fn tree_stream(&mut self, stream_id: StreamId) -> (impl Stream<Item = Tree>, Forest) {
        let me = stream_id.node_id() == self.node_id();
        if me {
            let stream_nr = stream_id.stream_nr();
            let stream = self.get_or_create_own_stream(stream_nr);
            (stream.tree_stream(), self.data.forest.clone())
        } else {
            let stream = self.get_or_create_replicated_stream(stream_id);
            (stream.tree_stream(), self.data.forest.clone())
        }
    }

    pub fn publish_new_stream_id(&mut self, stream_id: StreamId) {
        self.known_streams
            .retain(|sender| sender.unbounded_send(stream_id).is_ok())
    }

    pub fn current_stream_ids(&self) -> impl Iterator<Item = StreamId> + '_ {
        let node_id = self.data.node_id;
        let own_stream_ids = self.own_streams.keys().map(move |stream_id| node_id.stream(*stream_id));
        let replicated_stream_ids = self.remote_nodes.iter().flat_map(|(node_id, node_info)| {
            node_info
                .streams
                .keys()
                .map(move |stream_nr| node_id.stream(*stream_nr))
        });
        own_stream_ids.chain(replicated_stream_ids)
    }

    /// Get a complete root map from both own and replicated streams
    pub fn root_map(&self) -> BTreeMap<StreamId, Cid> {
        let own = self.own_streams.iter().filter_map(|(stream_nr, inner)| {
            let stream_id = self.node_id().stream(*stream_nr);
            inner.snapshot().cid().map(|root| (stream_id, root))
        });

        let other = self.remote_nodes.iter().flat_map(|(node_id, remote_node)| {
            remote_node.streams.iter().filter_map(move |(stream_nr, inner)| {
                let stream_id = node_id.stream(*stream_nr);
                inner.root().map(|root| (stream_id, root))
            })
        });
        own.chain(other).collect()
    }

    pub fn get_or_create_remote_node(&mut self, node_id: NodeId) -> &mut RemoteNodeInner {
        self.remote_nodes.entry(node_id).or_insert_with(|| {
            tracing::debug!("learned of new node {}", node_id);
            Default::default()
        })
    }

    /// Spawns a new task that will be shutdown when [`BanyanStore`] is dropped.
    pub fn spawn_task(&mut self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) {
        tracing::debug!("Spawning task '{}'!", name);
        let handle =
            tokio::spawn(task.map(move |_| tracing::error!("Fatal: Task '{}' unexpectedly terminated!", name)));
        self.tasks.push(handle);
    }

    /// reserve a number of lamport timestamps
    pub fn reserve_lamports(&mut self, n: usize) -> anyhow::Result<impl Iterator<Item = LamportTimestamp>> {
        let n = u64::try_from(n)?;
        let last_lamport = self.index_store.increase_lamport(n)?;
        Ok((last_lamport - n + 1..=last_lamport).map(LamportTimestamp::from))
    }
}

impl BanyanStore {
    /// Creates a new [`BanyanStore`] from a [`SwarmConfig`].
    pub async fn new(cfg: SwarmConfig) -> Result<Self> {
        tracing::debug!("client_from_config({:?})", cfg);
        tracing::debug!("Start listening on topic '{}'", &cfg.topic);

        let keypair = cfg.keypair.unwrap_or_else(KeyPair::generate);
        let node_id = keypair.into();
        let node_key: ipfs_embed::Keypair = keypair.into();
        let public = node_key.to_public();
        let node_name = cfg
            .node_name
            .unwrap_or_else(|| names::Generator::with_naming(names::Name::Numbered).next().unwrap());

        let ipfs = Ipfs::new(IpfsConfig {
            network: NetworkConfig {
                node_key,
                node_name,
                psk: cfg.psk,
                quic: Default::default(),
                mdns: if cfg.enable_mdns {
                    Some(Default::default())
                } else {
                    None
                },
                kad: None,
                dns: if cfg!(target_os = "android") {
                    // No official support for DNS on Android.
                    // see https://github.com/Actyx/Cosmos/issues/6582
                    Some(DnsConfig {
                        config: ResolverConfig::cloudflare(),
                        opts: Default::default(),
                    })
                } else {
                    None
                },
                ping: Some(
                    PingConfig::new()
                        .with_keep_alive(true)
                        .with_max_failures(NonZeroU32::new(2).unwrap()),
                ),
                identify: Some(IdentifyConfig::new("/actyx/2.0.0".to_string(), public)),
                gossipsub: if cfg.enable_root_map || cfg.enable_slow_path {
                    Some(
                        GossipsubConfigBuilder::default()
                            .validation_mode(ValidationMode::Permissive)
                            .build()
                            .expect("valid gossipsub config"),
                    )
                } else {
                    None
                },
                broadcast: if cfg.enable_fast_path {
                    Some(Default::default())
                } else {
                    None
                },
                bitswap: Some(BitswapConfig {
                    request_timeout: Duration::from_secs(10),
                    connection_keep_alive: Duration::from_secs(10),
                }),
            },
            storage: StorageConfig {
                path: cfg.db_path,
                cache_size_blocks: u64::MAX,
                cache_size_bytes: 1024 * 1024 * 1024 * 4,
                gc_interval: Duration::from_secs(10),
                gc_min_blocks: 1000,
                gc_target_duration: Duration::from_millis(10),
            },
        })
        .await?;

        let index_store = if let Some(conn) = cfg.index_store {
            SqliteIndexStore::from_conn(conn)?
        } else {
            SqliteIndexStore::open(DbPath::Memory)?
        };
        let forest = Forest::new(SqliteStore::wrap(ipfs.clone()), BranchCache::<TT>::new(64 << 20));
        let gossip = Gossip::new(
            ipfs.clone(),
            node_id,
            cfg.topic.clone(),
            cfg.enable_fast_path,
            cfg.enable_slow_path,
        );
        let banyan = Self {
            data: Arc::new(BanyanStoreData {
                node_id,
                ipfs,
                gossip,
                forest,
                lamport: Default::default(),
                offsets: Default::default(),
            }),
            state: Arc::new(ReentrantSafeMutex::new(BanyanStoreState {
                index_store,
                own_streams: Default::default(),
                remote_nodes: Default::default(),
                known_streams: Default::default(),
                tasks: Default::default(),
            })),
        };
        banyan.load_known_streams()?;
        banyan.spawn_task(
            "gossip_ingest",
            banyan.data.gossip.ingest(banyan.clone(), cfg.topic.clone())?,
        );
        if cfg.enable_root_map {
            banyan.spawn_task(
                "gossip_publish_root_map",
                banyan
                    .data
                    .gossip
                    .publish_root_map(banyan.clone(), cfg.topic.clone(), Duration::from_secs(10)),
            );
        }
        banyan.spawn_task("compaction", banyan.clone().compaction_loop(Duration::from_secs(60)));
        if cfg.enable_discovery {
            banyan.spawn_task("discovery_ingest", crate::discovery::discovery_ingest(banyan.clone()));
        }
        banyan.spawn_task(
            "discovery_publish",
            crate::discovery::discovery_publish(
                banyan.clone(),
                DISCOVERY_STREAM_NR.into(),
                cfg.external_addresses.iter().cloned().collect(),
                cfg.enable_discovery,
            )?,
        );
        if cfg.enable_metrics {
            banyan.spawn_task(
                "metrics",
                crate::metrics::metrics(banyan.clone(), METRICS_STREAM_NR.into(), Duration::from_secs(30))?,
            );
        }
        banyan.spawn_task(
            "prune_events",
            crate::prune::prune(banyan.clone(), cfg.ephemeral_event_config),
        );

        let ipfs = banyan.ipfs();
        for addr in cfg.listen_addresses {
            let port = addr
                .iter()
                .find_map(|x| match x {
                    Protocol::Tcp(p) => Some(p),
                    Protocol::Udp(p) => Some(p),
                    _ => None,
                })
                .unwrap_or_default();

            if let Some(ListenerEvent::NewListenAddr(bound_addr)) = ipfs
                .listen_on(addr.clone())
                .with_context(|| NodeErrorContext::BindFailed {
                    port,
                    component: "Swarm".into(),
                })?
                .next()
                .await
            {
                tracing::info!(target: "SWARM_SERVICES_BOUND", "Swarm Services bound to {}.", bound_addr);
            } else {
                return Err(anyhow::anyhow!("failed to bind address")).with_context(|| NodeErrorContext::BindFailed {
                    port,
                    component: "Swarm".into(),
                });
            }
        }
        for addr in cfg.external_addresses {
            ipfs.add_external_address(addr);
        }
        for mut addr in cfg.bootstrap_addresses {
            let addr_orig = addr.clone();
            if let Some(Protocol::P2p(peer_id)) = addr.pop() {
                let peer_id =
                    PeerId::from_multihash(peer_id).map_err(|_| anyhow::anyhow!("invalid bootstrap peer id"))?;
                if peer_id == ipfs.local_peer_id() {
                    tracing::warn!("Not dialing configured bootstrap node {} as it's myself", addr_orig);
                } else {
                    ipfs.dial_address(&peer_id, addr);
                }
            } else {
                return Err(anyhow::anyhow!("invalid bootstrap address"));
            }
        }

        Ok(banyan)
    }

    /// Creates a new [`BanyanStore`] for testing.
    pub async fn test(node_name: &str) -> Result<Self> {
        Self::new(SwarmConfig::test(node_name)).await
    }

    fn lock(&self) -> BanyanStoreGuard<'_> {
        BanyanStoreGuard {
            data: self.data.clone(),
            state: self.state.clone(),
            guard: self.state.lock(),
        }
    }

    fn load_known_streams(&self) -> Result<()> {
        let known_streams = self.lock().index_store.get_observed_streams()?;
        for stream_id in known_streams {
            // just trigger loading of the stream from the alias
            if stream_id.node_id() == self.node_id() {
                let _ = self.get_or_create_own_stream(stream_id.stream_nr());
            } else {
                let _ = self.get_or_create_replicated_stream(stream_id);
            }
        }
        Ok(())
    }

    /// Returns the [`NodeId`].
    pub fn node_id(&self) -> NodeId {
        self.data.node_id
    }

    /// Returns the underlying [`Ipfs`].
    pub fn ipfs(&self) -> &Ipfs {
        &self.data.ipfs
    }

    pub fn cat(&self, cid: Cid, path: VecDeque<String>) -> impl Stream<Item = Result<Vec<u8>>> + Send {
        unixfsv1::UnixfsStream::new(unixfsv1::UnixfsDecoder::new(self.ipfs().clone(), cid, path))
    }

    /// Append events to a stream, publishing the new data.
    pub async fn append(&self, stream_nr: StreamNr, events: Vec<(TagSet, Event)>) -> Result<Option<Link>> {
        tracing::debug!("publishing {} events on stream {}", events.len(), stream_nr);
        let timestamp = Timestamp::now();
        let stream = self.get_or_create_own_stream(stream_nr);
        let mut guard = stream.lock().await;
        let mut store = self.lock();
        let lamports = store.reserve_lamports(events.len())?;
        let kvs = lamports
            .zip(events)
            .map(|(lamport, (tags, payload))| (AxKey::new(tags, lamport, timestamp), payload));
        self.transform_stream(&mut guard, |txn, tree| txn.extend_unpacked(tree, kvs))?;
        Ok(guard.snapshot().link())
    }

    /// Returns a [`Stream`] of known [`StreamId`].
    pub fn stream_known_streams(&self) -> impl Stream<Item = StreamId> + Send {
        let mut state = self.lock();
        let (s, r) = mpsc::unbounded();
        for stream_id in state.current_stream_ids() {
            let _ = s.unbounded_send(stream_id);
        }
        state.known_streams.push(s);
        r
    }

    /// Returns a [`Stream`] of events in causal order filtered with a [`Query`].
    pub fn stream_filtered_stream_ordered<Q: Query<TT> + Clone + 'static>(
        &self,
        query: Q,
    ) -> impl Stream<Item = Result<(u64, Key, Event)>> {
        let this = self.clone();
        self.stream_known_streams()
            .map(move |stream_id| this.stream_filtered_chunked(stream_id, 0..=u64::max_value(), query.clone()))
            .merge_unordered()
            .map_ok(|chunk| stream::iter(chunk.data).map(Ok))
            .try_flatten()
    }

    pub fn stream_filtered_chunked<Q: Query<TT> + Clone + 'static>(
        &self,
        stream_id: StreamId,
        range: RangeInclusive<u64>,
        query: Q,
    ) -> impl Stream<Item = Result<FilteredChunk<TT, Event, ()>>> {
        tracing::debug!("stream_filtered_chunked {}", stream_id);
        let (trees, forest) = self.tree_stream(stream_id);
        forest.stream_trees_chunked(query, trees, range, &|_| {})
    }

    pub fn stream_filtered_chunked_reverse<Q: Query<TT> + Clone + 'static>(
        &self,
        stream_id: StreamId,
        range: RangeInclusive<u64>,
        query: Q,
    ) -> impl Stream<Item = Result<FilteredChunk<TT, Event, ()>>> {
        let (trees, forest) = self.tree_stream(stream_id);
        forest.stream_trees_chunked_reverse(query, trees, range, &|_| {})
    }

    fn get_or_create_own_stream(&self, stream_nr: StreamNr) -> Arc<OwnStream> {
        self.lock().get_or_create_own_stream(stream_nr)
    }

    fn get_or_create_replicated_stream(&self, stream_id: StreamId) -> Arc<ReplicatedStreamInner> {
        self.lock().get_or_create_replicated_stream(stream_id)
    }

    fn transform_stream<T>(
        &self,
        stream: &mut OwnStreamGuard,
        f: impl FnOnce(&Transaction, &mut StreamBuilder<AxTrees>) -> Result<T> + Send,
    ) -> Result<T> {
        let writer = self.data.forest.store().write()?;
        let stream_nr = stream.stream_nr();
        let stream_id = self.node_id().stream(stream_nr);
        let prev = stream.snapshot();
        tracing::debug!("starting write transaction on stream {}", stream_nr);
        let txn = Transaction::new(self.data.forest.clone(), writer);
        // take a snapshot of the initial state
        let mut guard = stream.transaction();
        let res = f(&txn, &mut guard);
        if res.is_err() {
            // stream builder state will be reverted, except for the cipher offset
            return res;
        }
        let curr = guard.snapshot();
        if curr.link() == prev.link() {
            // nothing to do, return
            return res;
        }
        // make sure we did not lose events. If we did, return a failure
        anyhow::ensure!(curr.count() >= prev.count(), "tree rejected because it lost events!");
        let cid = curr.link().map(Cid::from);
        // update the permanent alias. If this fails, we will revert the builder.
        self.ipfs().alias(StreamAlias::from(stream_id), cid.as_ref())?;
        // this concludes the things we want to fail the transaction
        guard.commit();
        // set the latest
        stream.latest().set(curr.clone());
        // update resent for the stream
        self.update_present(stream_id, curr.offset());
        // publish only non-empty trees
        if let Some(root) = curr.link() {
            // publish the update
            let blocks = txn.into_writer().into_written();
            // publish new blocks and root
            self.data.gossip.publish(stream_nr, root, blocks)?;
        }
        res
    }

    fn update_root(&self, stream_id: StreamId, root: Link) {
        if stream_id.node_id() != self.node_id() {
            tracing::trace!("update_root {} {}", stream_id, root);
            self.get_or_create_replicated_stream(stream_id).set_incoming(root);
        }
    }

    async fn compaction_loop(self, interval: Duration) {
        loop {
            let stream_nrs = self.lock().local_stream_nrs();
            for stream_nr in stream_nrs {
                tracing::debug!("compacting stream {}", stream_nr);
                let stream = self.get_or_create_own_stream(stream_nr);
                let mut guard = stream.lock().await;
                let result = self.transform_stream(&mut guard, |txn, tree| txn.pack(tree));
                if let Err(err) = result {
                    tracing::error!("Error compacting stream {}: {}", stream_nr, err);
                    break;
                }
            }
            tokio::time::sleep(interval).await;
        }
    }

    /// careful ingestion - basically just call sync_one on each new ingested root
    async fn careful_ingestion(self, stream_id: StreamId, state: Arc<ReplicatedStreamInner>) {
        state
            .incoming_root_stream()
            .switch_map(move |root| self.clone().sync_one(stream_id, root).into_stream())
            .for_each(|res| {
                if let Err(err) = res {
                    tracing::error!("careful_ingestion: {}", err);
                }
                future::ready(())
            })
            .await
    }

    /// attempt to sync one stream to a new root.
    ///
    /// this future may be interrupted at any time when an even newer root comes along.
    async fn sync_one(self, stream_id: StreamId, root: Link) -> Result<()> {
        let node_name = self.ipfs().local_node_name();
        // tokio::time::delay_for(Duration::from_millis(10)).await;
        tracing::debug!("starting to sync {} to {}", stream_id, root);
        let cid = Cid::from(root);
        let ipfs = &self.data.ipfs;
        let stream = self.get_or_create_replicated_stream(stream_id);
        let validated_lamport = stream.validated().last_lamport();
        // temporarily pin the new root
        tracing::debug!("assigning temp pin to {}", root);
        let temp_pin = ipfs.create_temp_pin()?;
        ipfs.temp_pin(&temp_pin, &cid)?;
        let peers = ipfs.peers();
        // attempt to sync. This may take a while and is likely to be interrupted
        tracing::debug!("starting to sync {} from {} peers", root, peers.len());
        // create the sync stream, and log progress. Add an additional element.
        let mut sync = ipfs.sync(&cid, peers);
        // during the sync, try to load the tree asap and abort in case it is not good
        let mut tree: Option<Tree> = None;
        let mut n: usize = 0;
        while let Some(event) = sync.next().await {
            match event {
                SyncEvent::Progress { missing } => {
                    tracing::debug!("{} sync_one: {}/{}", node_name, n, n + missing);
                    n += 1;
                }
                SyncEvent::Complete(Err(err)) => {
                    tracing::debug!("{} {}", node_name, err);
                    return Err(err);
                }
                SyncEvent::Complete(Ok(())) => {}
            }
            if tree.is_none() {
                // load the tree as soon as possible. If this fails, bail out.
                let temp = self.data.forest.load_tree(Secrets::default(), root)?;
                // check that the tree is better than the one we have, otherwise bail out
                anyhow::ensure!(temp.last_lamport() > validated_lamport);
                // get the offset
                let offset = temp.offset();
                // update present.
                let _ = self.update_highest_seen(stream_id, offset);
                tree = Some(temp);
            }
        }
        let tree = tree.ok_or_else(|| anyhow::anyhow!("unable to load tree"))?;

        // if we get here, we already know that the new tree is better than its predecessor
        tracing::debug!("completed sync of {}", root);
        // once sync is successful, permanently move the alias
        tracing::debug!("updating alias {}", root);
        // assign the new root as validated
        ipfs.alias(&StreamAlias::from(stream_id), Some(&cid))?;
        self.lock().received_lamport(tree.last_lamport().into())?;
        tracing::debug!("sync_one complete {} => {}", stream_id, tree.offset());
        let offset = tree.offset();
        stream.set_latest(tree);
        // update present. This can fail if stream_id is not a source_id.
        let _ = self.update_present(stream_id, offset);
        // done
        Ok(())
    }

    fn has_stream(&self, stream_id: StreamId) -> bool {
        self.lock().has_stream(stream_id)
    }

    /// stream of latest updates from either gossip (for replicated streams) or internal updates
    ///
    /// note that this does not include event updates
    fn latest_stream(&self, stream_id: StreamId) -> impl Stream<Item = (LamportTimestamp, Offset)> {
        self.lock().latest_stream(stream_id)
    }

    /// Get a stream of trees for a given stream id
    fn tree_stream(&self, stream_id: StreamId) -> (impl Stream<Item = Tree>, Forest) {
        self.lock().tree_stream(stream_id)
    }

    pub fn spawn_task(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) {
        self.lock().spawn_task(name, task)
    }
}

trait AxTreeExt {
    fn last_lamport(&self) -> LamportTimestamp;
    fn offset(&self) -> OffsetOrMin;
    fn cid(&self) -> Option<Cid>;
}

impl AxTreeExt for Tree {
    fn cid(&self) -> Option<Cid> {
        self.link().map(Into::into)
    }
    fn last_lamport(&self) -> LamportTimestamp {
        match self.as_index_ref() {
            Some(Index::Branch(branch)) => branch.summaries.lamport_range().max,
            Some(Index::Leaf(leaf)) => leaf.keys.lamport_range().max,
            None => Default::default(),
        }
    }
    fn offset(&self) -> OffsetOrMin {
        match self.count() {
            0 => OffsetOrMin::MIN,
            x => match u32::try_from(x) {
                Ok(fits) => (fits - 1).into(),
                Err(e) => {
                    tracing::error!("Tree's count ({}) is too big to fit into an offset ({})", x, e);
                    OffsetOrMin::MAX
                }
            },
        }
    }
}
