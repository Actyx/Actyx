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

pub mod blob_store;
mod discovery;
pub mod event_store;
pub mod event_store_ref;
mod gossip;
mod gossip_protocol;
pub mod metrics;
mod prune;
pub mod selection;
mod sqlite;
mod sqlite_index_store;
mod streams;
pub mod transport;

#[cfg(test)]
mod tests;

pub use crate::swarm::{
    gossip_protocol::{GossipMessage, RootMap, RootUpdate},
    sqlite::{StorageServiceStore, StorageServiceStoreWrite},
    sqlite_index_store::DbPath,
    streams::StreamAlias,
};
use crate::{
    ax_futures_util::stream::{
        variable::{Observer, Variable},
        AxStreamExt,
    },
    crypto::KeyPair,
    swarm::{
        event_store::PersistenceMeta,
        gossip::Gossip,
        sqlite::{SqliteStore, SqliteStoreWrite},
        streams::{OwnStream, PublishedTree, ReplicatedStream},
    },
    trees::{
        axtrees::{AxKey, AxTrees, Sha256Digest},
        dnf::Dnf,
        query::TagExprQuery,
        tags::{ScopedTag, ScopedTagSet},
        AxTree, AxTreeHeader,
    },
    util::{
        formats::NodeErrorContext,
        reentrant_safe_mutex::{ReentrantSafeMutex, ReentrantSafeMutexGuard},
        to_multiaddr, to_socket_addr, SocketAddrHelper,
    },
};
use anyhow::{Context, Result};
use ax_aql::{TagAtom, TagExpr};
use ax_types::{
    app_id, tag, AppId, LamportTimestamp, NodeId, Offset, OffsetMap, Payload, StreamId, StreamNr, TagSet, Timestamp,
};
use banyan::{
    query::Query,
    store::{BranchCache, ReadOnlyStore},
    FilteredChunk, Secrets,
};
pub use banyan::{store::BlockWriter, Forest as BanyanForest, StreamBuilder, Transaction as BanyanTransaction};
use fnv::FnvHashMap;
use futures::{
    channel::mpsc,
    future::{self, BoxFuture},
    stream, FutureExt, Stream, StreamExt, TryStreamExt,
};
use ipfs_embed::{
    config::BitswapConfig, identity::PublicKey::Ed25519, Cid, Config as IpfsConfig, DnsConfig, ListenerEvent,
    Multiaddr, NetworkConfig, PeerId, SyncEvent, TempPin,
};
pub use ipfs_embed::{Executor as IpfsEmbedExecutor, StorageConfig, StorageService};
pub use libipld::codec::Codec as IpldCodec;
use libipld::{cbor::DagCborCodec, error::BlockNotFound};
use libp2p::{
    dns::ResolverConfig,
    gossipsub::{GossipsubConfigBuilder, ValidationMode},
    identify,
    multiaddr::Protocol,
    ping,
};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
pub use prune::{RetainConfig, StreamAge, StreamSize};
use serde::{Deserialize, Serialize};
use sqlite_index_store::SqliteIndexStore;
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    convert::{TryFrom, TryInto},
    fmt::{Debug, Display},
    io::{BufRead, BufReader, Read},
    num::NonZeroU32,
    ops::{Deref, DerefMut, RangeInclusive},
    path::PathBuf,
    process::Command,
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use streams::{OwnStreamGuard, RemoteNodeInner};
pub use unixfs_v1::{
    dir::builder::{BufferingTreeBuilder, TreeOptions},
    FlatUnixFs, PBLink, UnixFsType,
};
use unixfs_v1::{
    dir::MaybeResolved,
    file::{
        adder::FileAdder,
        visit::{FileVisit, IdleFileVisit},
    },
};

#[allow(clippy::upper_case_acronyms)]
pub type TT = AxTrees;
pub type Key = AxKey;
pub type Event = Payload;
pub type Forest = banyan::Forest<TT, SqliteStore>;
pub type Transaction = banyan::Transaction<TT, SqliteStore, SqliteStoreWrite>;
pub type Tree = banyan::Tree<TT, Event>;
pub type AxStreamBuilder = banyan::StreamBuilder<TT, Event>;
pub type Link = Sha256Digest;

pub use crate::trees::StoreParams;
use acto::ActoRef;
pub type Block = libipld::Block<StoreParams>;
pub type Ipfs = ipfs_embed::Ipfs<StoreParams>;

const MAX_TREE_LEVEL: i32 = 512;

const DEFAULT_STREAM_NAME: &str = "default";
const DEFAULT_STREAM_NUMBER: u64 = 0;

const DISCOVERY_STREAM_NAME: &str = "discovery";
const DISCOVERY_STREAM_NUMBER: u64 = 1;

const METRICS_STREAM_NAME: &str = "metrics";
const METRICS_STREAM_NUMBER: u64 = 2;

const FILES_STREAM_NAME: &str = "files";
const FILES_STREAM_NUMBER: u64 = 3;

const EVENT_ROUTING_TAG_NAME: &str = "event_routing";

/// The default pruning interval (in seconds).
const DEFAULT_PRUNING_INTERVAL: u64 = 30 * 60;

fn internal_app_id() -> AppId {
    app_id!("com.actyx")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EphemeralEventsConfig {
    interval: Duration,
    streams: BTreeMap<String, RetainConfig>,
}

impl EphemeralEventsConfig {
    pub fn new(interval: Duration, streams: BTreeMap<String, RetainConfig>) -> Self {
        Self { interval, streams }
    }
    pub fn disable() -> Self {
        Self {
            streams: BTreeMap::default(),
            interval: Duration::from_secs(u64::MAX),
        }
    }
}

impl Default for EphemeralEventsConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(DEFAULT_PRUNING_INTERVAL),
            streams: BTreeMap::new(),
        }
    }
}

impl FromStr for EphemeralEventsConfig {
    type Err = serde_json::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

impl From<BTreeMap<String, RetainConfig>> for EphemeralEventsConfig {
    fn from(streams: BTreeMap<String, RetainConfig>) -> Self {
        Self {
            interval: Duration::from_secs(DEFAULT_PRUNING_INTERVAL),
            streams,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SwarmConfig {
    pub topic: String,
    pub index_store: Option<PathBuf>,
    pub blob_store: Option<PathBuf>,
    pub keypair: Option<KeyPair>,
    pub psk: Option<[u8; 32]>,
    pub node_name: Option<String>,
    pub db_path: Option<PathBuf>,
    pub block_cache_size: u64,
    pub block_cache_count: u64,
    pub block_gc_interval: Duration,
    pub external_addresses: Vec<Multiaddr>,
    pub listen_addresses: Arc<Mutex<SocketAddrHelper>>,
    pub bootstrap_addresses: Vec<Multiaddr>,
    pub ephemeral_event_config: EphemeralEventsConfig,
    pub enable_loopback: bool,
    pub enable_fast_path: bool,
    pub enable_slow_path: bool,
    pub enable_mdns: bool,
    pub enable_root_map: bool,
    pub enable_discovery: bool,
    pub enable_metrics: bool,
    pub banyan_config: BanyanConfig,
    pub cadence_root_map: Duration,
    pub cadence_compact: Duration,
    pub metrics_interval: Duration,
    pub ping_timeout: Duration,
    pub bitswap_timeout: Duration,
    pub branch_cache_size: u64,
    pub event_routes: Vec<EventRoute>,
}
impl SwarmConfig {
    pub fn basic() -> Self {
        Self {
            enable_loopback: false,
            topic: String::from("default"),
            index_store: None,
            blob_store: None,
            keypair: None,
            psk: None,
            node_name: None,
            db_path: None,
            external_addresses: vec![],
            listen_addresses: Arc::new(Mutex::new(SocketAddrHelper::empty())),
            bootstrap_addresses: vec![],
            ephemeral_event_config: EphemeralEventsConfig::default(),
            enable_fast_path: true,
            enable_slow_path: true,
            enable_mdns: true,
            enable_root_map: true,
            enable_discovery: true,
            enable_metrics: true,
            banyan_config: BanyanConfig::default(),
            cadence_compact: Duration::from_secs(60),
            cadence_root_map: Duration::from_secs(10),
            block_cache_size: 1024 * 1024 * 1024,
            block_cache_count: 1024 * 128,
            block_gc_interval: Duration::from_secs(300),
            metrics_interval: Duration::from_secs(60 * 30),
            ping_timeout: Duration::from_secs(5),
            bitswap_timeout: Duration::from_secs(15),
            branch_cache_size: 67108864,
            event_routes: Default::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BanyanConfig {
    pub tree: banyan::Config,
    pub secret: banyan::Secrets,
}
impl Default for BanyanConfig {
    fn default() -> Self {
        // reasoning: up to 16384 events per leaf, makes up to 65536 event keys per level 1 block
        // this permits up to 30 bytes (avg) of compressed per-event key data (2MiB max block size)
        // target leaf size only guards against huge events
        let tree = banyan::Config {
            max_key_branches: 4,
            max_leaf_count: 16384,
            target_leaf_size: 1_000_000,
            ..banyan::Config::debug_fast()
        };
        Self {
            tree,
            secret: banyan::Secrets::default(),
        }
    }
}

impl SwarmConfig {
    pub fn test(node_name: &str) -> Self {
        Self {
            enable_loopback: true,
            topic: "topic".into(),
            enable_mdns: false,
            node_name: Some(node_name.into()),
            listen_addresses: Arc::new(Mutex::new("127.0.0.1:0".parse().unwrap())),
            banyan_config: BanyanConfig {
                tree: banyan::Config::debug(),
                ..Default::default()
            },
            ..SwarmConfig::basic()
        }
    }

    pub fn test_with_routing(node_name: &str, routes: Vec<EventRoute>) -> Self {
        Self {
            event_routes: routes,
            ..SwarmConfig::test(node_name)
        }
    }
}

impl PartialEq for SwarmConfig {
    fn eq(&self, other: &Self) -> bool {
        // can’t hold both locks because that would deadlock with another
        // comparison on another thread
        let me_listen = self.listen_addresses.lock().clone();
        let they_listen = other.listen_addresses.lock().clone();
        self.topic == other.topic
            && self.keypair == other.keypair
            && self.psk == other.psk
            && self.node_name == other.node_name
            && self.db_path == other.db_path
            && self.block_cache_size == other.block_cache_size
            && self.block_cache_count == other.block_cache_count
            && self.block_gc_interval == other.block_gc_interval
            && self.external_addresses == other.external_addresses
            && me_listen == they_listen
            && self.bootstrap_addresses == other.bootstrap_addresses
            && self.ephemeral_event_config == other.ephemeral_event_config
            && self.enable_loopback == other.enable_loopback
            && self.enable_fast_path == other.enable_fast_path
            && self.enable_slow_path == other.enable_slow_path
            && self.enable_mdns == other.enable_mdns
            && self.enable_root_map == other.enable_root_map
            && self.enable_discovery == other.enable_discovery
            && self.enable_metrics == other.enable_metrics
            && self.cadence_root_map == other.cadence_root_map
            && self.cadence_compact == other.cadence_compact
            && self.metrics_interval == other.metrics_interval
            && self.ping_timeout == other.ping_timeout
            && self.bitswap_timeout == other.bitswap_timeout
            && self.branch_cache_size == other.branch_cache_size
            && self.event_routes == other.event_routes
    }
}

/// Stream manager.
#[derive(Clone)]
pub struct BanyanStore {
    data: Arc<BanyanStoreData>,
    state: Arc<ReentrantSafeMutex<BanyanStoreState>>,
}

#[derive(Clone, Debug, Default)]
pub struct SwarmOffsets {
    /// Currently validated OffsetMap
    present: OffsetMap,
    /// OffsetMap describing the replication target. Currently this is driven via `highest_seen`,
    /// but should eventually be fed by the partial replication mechanism.
    replication_target: OffsetMap,
}

impl SwarmOffsets {
    /// Currently validated OffsetMap
    pub fn present(&self) -> OffsetMap {
        self.present.clone()
    }

    /// OffsetMap describing the replication target. Currently this is driven via `highest_seen`,
    /// but should eventually be fed by the partial replication mechanism.
    pub fn replication_target(&self) -> OffsetMap {
        self.replication_target.clone()
    }
}

pub struct AppendMeta {
    min_lamport: LamportTimestamp,
    min_offset: Offset,
    timestamp: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd)]
pub struct RootSource {
    path: RootPath,
    sender: PeerId,
}

impl RootSource {
    fn new(sender: PeerId, path: RootPath) -> Self {
        Self { sender, path }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RootPath {
    // needs to be ordered in ascending priority
    RootMap,
    SlowPath,
    FastPath,
}

#[test]
fn root_path_is_ordered() {
    use RootPath::*;
    assert!(RootMap < SlowPath);
    assert!(SlowPath < FastPath);

    assert!(RootSource::new(PeerId::random(), RootMap) < RootSource::new(PeerId::random(), SlowPath));
    assert!(RootSource::new(PeerId::random(), SlowPath) < RootSource::new(PeerId::random(), FastPath));
}

/// All immutable or internally mutable parts of the banyan store
struct BanyanStoreData {
    topic: String,
    gossip: Gossip,
    forest: Forest,
    ipfs: Ipfs,
    node_id: NodeId,
    /// maximum ingested offset and highest seen for each stream
    offsets: Variable<SwarmOffsets>,
    /// lamport timestamp for publishing to internal streams
    lamport: Observer<LamportTimestamp>,
    /// Routing table
    routing_table: Lazy<RoutingTable, Box<dyn FnOnce() -> RoutingTable + Send>>,
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
    tasks: Vec<(String, tokio::task::JoinHandle<()>)>,

    /// Banyan related config
    banyan_config: BanyanConfig,
}

impl Drop for BanyanStoreState {
    fn drop(&mut self) {
        for (name, task) in self.tasks.drain(..) {
            tracing::debug!("Banyan drop aborting task {}", name);
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

    fn get_or_create_own_stream(&mut self, stream_nr: StreamNr) -> Result<Arc<OwnStream>> {
        if let Some(result) = self.own_streams.get(&stream_nr).cloned() {
            return Ok(result);
        }
        tracing::debug!("creating new own stream {}", stream_nr);
        let stream_id = self.node_id().stream(stream_nr);
        self.index_store
            .add_stream(stream_id)
            .context("unable to write stream id")?;
        let (builder, latest) = if let Some(root) = self
            .data
            .ipfs
            .resolve(StreamAlias::from(stream_id))
            .context("no alias for stream id")?
        {
            let root = Link::try_from(root).context("wrong link format")?;
            let header = self.data.forest.store().get(&root).context("header not found")?;
            let header: AxTreeHeader = DagCborCodec.decode(&header).context("invalid header")?;
            let builder = self
                .data
                .forest
                .load_stream_builder(
                    self.banyan_config.secret.clone(),
                    self.banyan_config.tree.clone(),
                    header.root,
                )
                .with_context(|| format!("unable to load banyan tree for stream {}", stream_nr))?;
            let published = PublishedTree::new(root, header, builder.snapshot());
            (builder, Some(published))
        } else {
            let builder = StreamBuilder::new(self.banyan_config.tree.clone(), self.banyan_config.secret.clone());
            (builder, None)
        };
        let stream = Arc::new(OwnStream::new(stream_nr, builder, latest));
        self.own_streams.insert(stream_nr, stream.clone());
        tracing::debug!("publish new stream_id {}", stream_id);
        self.publish_new_stream_id(stream_id);
        Ok(stream)
    }

    fn get_or_create_replicated_stream(&mut self, stream_id: StreamId) -> Result<Arc<ReplicatedStream>> {
        debug_assert!(!self.is_local(stream_id));
        self.index_store
            .add_stream(stream_id)
            .context("unable to write stream id")?;
        let node_id = stream_id.node_id();
        let stream_nr = stream_id.stream_nr();
        if let Some(stream) = self.get_or_create_remote_node(node_id).streams.get(&stream_nr).cloned() {
            return Ok(stream);
        }
        let state = if let Some(root) = self.data.ipfs.resolve(StreamAlias::from(stream_id)).unwrap() {
            let root = Link::try_from(root).context("wrong link format")?;
            let header = self.data.forest.store().get(&root).context("header not found")?;
            let header: AxTreeHeader = DagCborCodec.decode(&header).context("invalid header")?;
            let tree = self
                .data
                .forest
                .load_tree(Secrets::default(), header.root)
                .with_context(|| format!("unable to load banyan tree for stream {}", stream_id))?;
            Some(PublishedTree::new(root, header, tree))
        } else {
            None
        };
        tracing::debug!("creating new replicated stream {}", stream_id);
        let stream = Arc::new(ReplicatedStream::new(state));
        self.get_or_create_remote_node(node_id)
            .streams
            .insert(stream_nr, stream.clone());
        let store = self.outer();
        self.spawn_task(
            format!("careful_ingestion({})", stream_id),
            store.careful_ingestion(stream_id, stream.clone()).boxed(),
        );
        tracing::debug!("publish new stream_id {}", stream_id);
        self.publish_new_stream_id(stream_id);
        Ok(stream)
    }

    fn is_local(&self, stream_id: StreamId) -> bool {
        stream_id.node_id() == self.node_id()
    }

    fn has_stream(&self, stream_id: StreamId) -> bool {
        if self.is_local(stream_id) {
            self.own_streams.contains_key(&stream_id.stream_nr())
        } else {
            self.remote_nodes
                .get(&stream_id.node_id())
                .map(|node| node.streams.contains_key(&stream_id.stream_nr()))
                .unwrap_or_default()
        }
    }

    /// Get the last PublishedTree for a stream_id, only if it already exists
    fn published_tree(&self, stream_id: StreamId) -> Option<PublishedTree> {
        if self.is_local(stream_id) {
            let stream_nr = stream_id.stream_nr();
            let stream = self.own_streams.get(&stream_nr)?;
            stream.published_tree()
        } else {
            let node_id = stream_id.node_id();
            let stream_nr = stream_id.stream_nr();
            let remote = self.remote_nodes.get(&node_id)?;
            let stream = remote.streams.get(&stream_nr)?;
            stream.latest()
        }
    }

    /// Get a stream of trees for a given stream id
    fn tree_stream(&mut self, stream_id: StreamId) -> impl Stream<Item = Tree> {
        if self.is_local(stream_id) {
            let stream_nr = stream_id.stream_nr();
            let stream = self.get_or_create_own_stream(stream_nr).unwrap();
            stream.tree_stream()
        } else {
            let stream = self.get_or_create_replicated_stream(stream_id).unwrap();
            stream.tree_stream()
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
    pub fn root_map(&self) -> BTreeMap<StreamId, (Cid, Offset, LamportTimestamp)> {
        let own = self.own_streams.iter().filter_map(|(stream_nr, inner)| {
            let stream_id = self.node_id().stream(*stream_nr);
            inner.infos().map(|infos| (stream_id, infos))
        });

        let other = self.remote_nodes.iter().flat_map(|(node_id, remote_node)| {
            remote_node.streams.iter().filter_map(move |(stream_nr, inner)| {
                let stream_id = node_id.stream(*stream_nr);
                inner.infos().map(|infos| (stream_id, infos))
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
    pub fn spawn_task(&mut self, name: String, task: BoxFuture<'static, ()>) {
        tracing::debug!("Spawning task '{}'!", name);
        let name2 = name.clone();
        let handle =
            tokio::spawn(task.map(move |_| tracing::error!("Fatal: Task '{}' unexpectedly terminated!", name2)));
        self.tasks.push((name, handle));
    }

    /// Aborts a task.
    pub fn abort_task(&mut self, name: &'static str) {
        self.tasks.retain(|(label, handle)| {
            if *label == name {
                handle.abort();
                false
            } else {
                true
            }
        })
    }

    /// reserve a number of lamport timestamps
    fn reserve_lamports(&mut self, n: usize) -> anyhow::Result<impl Iterator<Item = LamportTimestamp>> {
        let n = u64::try_from(n)?;
        let initial = self.index_store.increase_lamport(n)?;
        Ok((u64::from(initial)..u64::from(initial + n)).map(LamportTimestamp::from))
    }

    fn received_lamport(&mut self, lamport: LamportTimestamp) -> anyhow::Result<()> {
        self.index_store.received_lamport(lamport).map_err(|e| {
            if let Err(e) = Command::new("/bin/ls").args(["-l", "/proc/self/fd"]).spawn() {
                tracing::error!("error checking file descriptors: {}", e);
            }
            e
        })
    }

    /// Compute the swarm offsets from scratch based on the in memory headers and trees
    fn compute_swarm_offsets(&self) -> SwarmOffsets {
        let mut present = OffsetMap::empty();
        for stream_id in self.current_stream_ids() {
            if let Some(tree) = self.published_tree(stream_id) {
                present.update(stream_id, tree.offset());
            }
        }
        SwarmOffsets {
            replication_target: present.clone(),
            present,
        }
    }

    fn load_known_streams(&mut self) -> Result<u64> {
        let known_streams = self.index_store.get_observed_streams()?;
        let mut max_lamport = None;
        let mut local_streams = 0;
        for stream_id in known_streams {
            // just trigger loading of the stream from the alias
            // NOTE: I can return the number of local streams and then use it to check if I need to write the mappings
            // happens when no mappings are present + the number of streams is bigger than 1
            let lamport = if self.is_local(stream_id) {
                local_streams += 1;
                self.get_or_create_own_stream(stream_id.stream_nr())?
                    .infos()
                    .map(|x| x.2)
            } else {
                self.get_or_create_replicated_stream(stream_id)?.infos().map(|x| x.2)
            };
            max_lamport = max_lamport.max(lamport);
        }
        if let Some(lamport) = max_lamport {
            // register our lower bound on lamport just in case the meta table wasn’t there
            // (e.g. migrating from per-2.9)
            tracing::info!("propagating Lamport timestamp {} from store", lamport);
            self.received_lamport(lamport)?;
        }
        self.data.offsets.set(self.compute_swarm_offsets());
        Ok(local_streams)
    }
}

impl BanyanStore {
    /// Creates a new [`BanyanStore`] from a [`SwarmConfig`].
    pub async fn new(mut cfg: SwarmConfig, swarm_observer: ActoRef<(PeerId, GossipMessage)>) -> Result<Self> {
        tracing::debug!("client_from_config({:?})", cfg);
        tracing::debug!("Start listening on topic '{}'", &cfg.topic);

        let keypair = cfg.keypair.unwrap_or_else(KeyPair::generate);
        let node_id = keypair.into();
        let node_key: ipfs_embed::identity::ed25519::Keypair = keypair.into();
        let public = node_key.public();
        let node_name = cfg
            .node_name
            .unwrap_or_else(|| names::Generator::with_naming(names::Name::Numbered).next().unwrap());

        let mut ipfs = Ipfs::new(IpfsConfig {
            network: NetworkConfig {
                enable_loopback: cfg.enable_loopback,
                port_reuse: false,
                keep_alive: true,
                node_key,
                node_name: node_name.clone(),
                psk: cfg.psk,
                mdns: if cfg.enable_mdns {
                    Some(Default::default())
                } else {
                    None
                },
                kad: None,
                dns: if cfg!(target_os = "android") {
                    // No official support for DNS on Android.
                    // see https://github.com/Actyx/Cosmos/issues/6582
                    Some(DnsConfig::Custom {
                        config: ResolverConfig::cloudflare(),
                        opts: Default::default(),
                    })
                } else {
                    Some(DnsConfig::SystemWithFallback {
                        config: ResolverConfig::cloudflare(),
                        opts: Default::default(),
                    })
                },
                ping: Some(
                    ping::Config::new()
                        .with_interval(Duration::from_secs(20))
                        .with_timeout(cfg.ping_timeout)
                        .with_max_failures(NonZeroU32::new(3).unwrap()),
                ),
                identify: Some(
                    identify::Config::new("/actyx/2.0.0".to_string(), Ed25519(public)).with_agent_version(node_name),
                ),
                gossipsub: Some(
                    GossipsubConfigBuilder::default()
                        .validation_mode(ValidationMode::Permissive)
                        .build()
                        .expect("valid gossipsub config"),
                ),
                broadcast: Some(Default::default()),
                bitswap: Some(BitswapConfig {
                    request_timeout: cfg.bitswap_timeout,
                    connection_keep_alive: cfg.bitswap_timeout,
                }),
            },
            storage: StorageConfig {
                access_db_path: None, // in memory
                path: cfg.db_path,
                cache_size_blocks: cfg.block_cache_count,
                cache_size_bytes: cfg.block_cache_size,
                gc_interval: cfg.block_gc_interval,
                // with the duration below gc will keep running continuously
                // if need be, so no need for an effective minimum here
                gc_min_blocks: 1,
                // gc is concurrent to normal operations, so could run forever,
                // but we want to get stats now and then
                gc_target_duration: cfg.block_gc_interval,
            },
        })
        .await?;
        // call as soon as possible to avoid missed events
        let swarm_events = ipfs.swarm_events().await?;
        let mut bootstrap: FnvHashMap<PeerId, Vec<Multiaddr>> = FnvHashMap::default();
        for mut addr in cfg.bootstrap_addresses {
            tracing::debug!(addr = display(&addr), "adding initial peer");
            if let Some(Protocol::P2p(peer_id)) = addr.pop() {
                let peer_id =
                    PeerId::from_multihash(peer_id).map_err(|_| anyhow::anyhow!("invalid bootstrap peer id"))?;
                bootstrap.entry(peer_id).or_default().push(addr);
            } else {
                return Err(anyhow::anyhow!("invalid bootstrap address"));
            }
        }
        let listen_addrs = cfg.listen_addresses.lock().iter().collect::<Vec<_>>();
        for addr in listen_addrs {
            let maddr = to_multiaddr(addr);
            let mut listener = ipfs.listen_on(maddr.clone());

            match listener.next().await {
                Some(ListenerEvent::NewListenAddr(bound_addr)) => {
                    // we print only the first of the discovered addresses, but the others will also be found
                    tracing::info!(target: "SWARM_SERVICES_BOUND", "Swarm Services bound to {}.", bound_addr);
                    if let Some(bound_addr) = to_socket_addr(bound_addr) {
                        cfg.listen_addresses.lock().inject_bound_addr(addr, bound_addr);
                    }
                }
                Some(ListenerEvent::ListenFailed(_addr, reason)) => {
                    return Err(anyhow::anyhow!("bind failed: {}", reason)).with_context(|| {
                        NodeErrorContext::BindFailed {
                            addr: maddr.clone(),
                            component: "Swarm".into(),
                        }
                    })
                }
                e => {
                    return Err(anyhow::anyhow!("got unexpected event {:?}", e)).with_context(|| {
                        NodeErrorContext::BindFailed {
                            addr: maddr,
                            component: "Swarm".into(),
                        }
                    })
                }
            }

            // print the remaining listen addresses asynchronously
            tokio::spawn(async move {
                while let Some(ev) = listener.next().await {
                    match ev {
                        ListenerEvent::NewListenAddr(bound_addr) => {
                            tracing::info!(target: "SWARM_SERVICES_BOUND", "Swarm Services bound to {}.", bound_addr)
                        }
                        ListenerEvent::ExpiredListenAddr(addr) => {
                            tracing::info!("Swarm Services no longer listening on {}.", addr)
                        }
                        ListenerEvent::ListenFailed(addr, reason) => {
                            tracing::warn!(%addr, %reason, "got belated listen failure");
                        }
                    }
                }
            });
        }
        let external_addrs = cfg.external_addresses.iter().cloned().collect();
        for addr in cfg.external_addresses {
            ipfs.add_external_address(addr);
        }

        let peers = bootstrap.keys().cloned().collect::<Vec<_>>();
        for (peer, addrs) in bootstrap {
            for mut addr in addrs {
                ipfs.add_address(peer, addr.clone());
                let addr_dbg = tracing::field::debug(addr.clone());
                if let Some(info) = ipfs.peer_info(&peer) {
                    addr.push(Protocol::P2p(peer.into()));
                    if !info.addresses().any(|(a, ..)| *a == addr) {
                        tracing::warn!(id = display(peer), addr = addr_dbg, "failed to add initial peer");
                    } else {
                        tracing::info!(id = display(peer), addr = addr_dbg, "added initial peer");
                    }
                } else {
                    tracing::warn!(id = display(peer), addr = addr_dbg, "failed to add initial peer");
                }
            }
        }

        let index_store = if let Some(conn) = cfg.index_store {
            let mut db = SqliteIndexStore::open(DbPath::File(conn))?;
            if db.get_observed_streams()?.is_empty() {
                // either a new store or migrating from pre-2.9
                let aliases = ipfs.aliases()?;
                if !aliases.is_empty() {
                    tracing::info!("starting store migration from pre-2.9 or dump");
                    let aliases = aliases.into_iter().filter_map(|(alias, _cid)| {
                        let stream_alias = StreamAlias::try_from(alias.as_slice()).ok()?;
                        StreamId::try_from(stream_alias).ok()
                    });
                    let mut count = 0;
                    for stream in aliases {
                        tracing::debug!("migrating stream {}", stream);
                        db.add_stream(stream)?;
                        count += 1;
                    }
                    tracing::info!("migrated {} streams", count);
                }
            }
            db
        } else {
            SqliteIndexStore::open(DbPath::Memory)?
        };
        let branch_cache = BranchCache::<TT>::new(cfg.branch_cache_size.try_into().unwrap());
        let forest = Forest::new(SqliteStore::wrap(ipfs.clone()), branch_cache.clone());
        let gossip = Gossip::new(
            ipfs.clone(),
            node_id,
            cfg.topic.clone(),
            cfg.enable_fast_path,
            cfg.enable_slow_path,
            swarm_observer.clone(),
        );
        let routing_table_writer = Arc::new(Mutex::new(None));
        let routing_table_reader = routing_table_writer.clone();
        let banyan = Self {
            data: Arc::new(BanyanStoreData {
                topic: cfg.topic.clone(),
                node_id,
                ipfs,
                gossip,
                forest,
                lamport: index_store.observe_lamport(),
                offsets: Default::default(),
                routing_table: Lazy::new(Box::new(move || routing_table_reader.lock().take().unwrap())),
            }),
            state: Arc::new(ReentrantSafeMutex::new(BanyanStoreState {
                index_store,
                own_streams: Default::default(),
                remote_nodes: Default::default(),
                known_streams: Default::default(),
                tasks: Default::default(),
                banyan_config: cfg.banyan_config,
            })),
        };
        tracing::info!("loading event streams");
        let local_streams = banyan.lock().load_known_streams()?;
        // check that all known streams are indeed completely present
        tracing::info!("validating event streams");
        banyan.validate_known_streams().await?;

        let routing_table_span = tracing::debug_span!("Initializing routing table.");
        let known_mappings = banyan.get_published_mappings(node_id).await?;

        let routing_table_span_entered = routing_table_span.enter();
        let mut routing_table = RoutingTable::default();

        if known_mappings.is_empty() {
            tracing::info!("No stream mappings found, publishing the default mapping.");
            banyan
                .append_stream_mapping_event(DEFAULT_STREAM_NAME.to_string(), DEFAULT_STREAM_NUMBER.into())
                .await?;
            tracing::debug!("\"{}\" stream successfully published.", DEFAULT_STREAM_NAME);
            // If publishing went ok, we can move on with adding the stream to the table
            routing_table
                .add_stream(DEFAULT_STREAM_NAME, Some(DEFAULT_STREAM_NUMBER.into()))
                .expect("The stream should not have been previously added.");

            let extended_default_streams = [
                (
                    StreamNr::from(DISCOVERY_STREAM_NUMBER),
                    DISCOVERY_STREAM_NAME,
                    RetainConfig::events(1000),
                ),
                (
                    StreamNr::from(METRICS_STREAM_NUMBER),
                    METRICS_STREAM_NAME,
                    RetainConfig::events(1000),
                ),
                (
                    StreamNr::from(FILES_STREAM_NUMBER),
                    FILES_STREAM_NAME,
                    RetainConfig::age_from_seconds(60 * 60 * 24 * 14),
                ),
            ];
            // Only consider the event routes because the retain configs do not publish streams
            // and we should be able to configure retain policies for the old default mappings
            if cfg.event_routes.is_empty() || local_streams > 1 {
                for (stream_nr, stream_name, retain_cfg) in extended_default_streams {
                    banyan
                        .append_stream_mapping_event(stream_name.to_string(), stream_nr)
                        .await?;
                    tracing::debug!("\"{}\" stream successfully published.", stream_name);
                    // If publishing went ok, we can move on with adding the stream to the table
                    routing_table
                        .add_stream(stream_name, Some(stream_nr))
                        .expect("The stream should not have been previously added.");

                    cfg.ephemeral_event_config
                        .streams
                        .insert(stream_name.to_string(), retain_cfg);
                }
            }
        } else {
            for (name, nr) in known_mappings {
                routing_table.add_stream(&name, nr.into())?;
            }
        }

        let unpublished_mappings = {
            let mut unpublished_mappings = HashMap::new();

            for route in cfg.event_routes {
                if let Some(stream_nr) = routing_table.add_route(route.from, route.into.clone()) {
                    unpublished_mappings.insert(route.into, stream_nr);
                }
            }
            unpublished_mappings
        };

        cfg.ephemeral_event_config.streams = cfg
            .ephemeral_event_config
            .streams
            .into_iter()
            .filter(|(stream, _)| {
                if stream == "default" {
                    tracing::warn!(
                        "The \"default\" stream cannot be configured, its retention configuration will be ignored."
                    );
                    return false;
                }

                let is_stream_mapped = routing_table.stream_mapping.get(stream).is_some();
                if !is_stream_mapped {
                    tracing::warn!(
                        "The stream \"{}\" does not have a mapping, its retention configuration will be ignored.",
                        stream
                    );
                }
                is_stream_mapped
            })
            .collect::<_>();

        drop(routing_table_span_entered);
        for (name, number) in unpublished_mappings {
            banyan.append_stream_mapping_event(name, number).await?;
        }
        let routing_table_span_entered = routing_table_span.enter();
        *routing_table_writer.lock() = Some(routing_table);

        tracing::debug!("Finished setting up routing.");
        drop(routing_table_span_entered);
        drop(routing_table_span);

        tracing::info!("starting maintenance tasks");
        banyan.spawn_task(
            "cache_debug".to_owned(),
            async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    tracing::debug!(?branch_cache);
                }
            }
            .boxed(),
        );
        banyan.spawn_task(
            "gossip_ingest".to_owned(),
            Gossip::ingest(banyan.clone(), cfg.topic.clone(), swarm_observer.clone())
                .await?
                .boxed(),
        );
        if cfg.enable_root_map {
            banyan.spawn_task(
                "gossip_publish_root_map".to_owned(),
                banyan
                    .data
                    .gossip
                    .publish_root_map(banyan.clone(), cfg.topic.clone(), cfg.cadence_root_map, swarm_observer)
                    .boxed(),
            );
        }
        banyan.spawn_task(
            "compaction".to_owned(),
            banyan.clone().compaction_loop(cfg.cadence_compact).boxed(),
        );
        if cfg.enable_discovery {
            banyan.spawn_task(
                "discovery_ingest".to_owned(),
                discovery::discovery_ingest(banyan.clone()).boxed(),
            );
        }
        // if `cfg.enable_discovery` is not set, this function WON'T emit any
        // events! It's needed in any case for `ipfs-embed` to do its thing.
        banyan.spawn_task(
            "discovery".to_owned(),
            discovery::discovery_publish(
                banyan.clone(),
                swarm_events,
                external_addrs,
                cfg.enable_discovery,
                peers,
            )?
            .boxed(),
        );
        if cfg.enable_metrics {
            banyan.spawn_task(
                "metrics".to_owned(),
                metrics::metrics(banyan.clone(), cfg.metrics_interval)?.boxed(),
            );
        }

        banyan.spawn_task(
            "prune_events".to_owned(),
            prune::prune(banyan.clone(), cfg.ephemeral_event_config).boxed(),
        );

        Ok(banyan)
    }

    pub fn get_topic(&self) -> String {
        self.data.topic.clone()
    }

    /// Loads the default stream, reading all [RouteMappingEvents] from it and returning
    /// the respective route mapping.
    async fn get_published_mappings(&self, node_id: NodeId) -> Result<HashMap<String, StreamNr>> {
        let stream_id = node_id.stream(DEFAULT_STREAM_NUMBER.into());

        let tag_expr = format!("'{}' & appId({})", EVENT_ROUTING_TAG_NAME, internal_app_id());
        let tag_expr = TagExpr::from_str(&tag_expr).expect("The tag expression should be valid.");
        let query_factory = TagExprQuery::from_expr(&tag_expr).expect("The query should be valid.");
        // The boolean value we call here is irrelevant
        let query = query_factory(false, stream_id);

        let published_tree = self.lock().published_tree(stream_id);
        if let Some(tree) = published_tree {
            let offset = tree.offset().into();
            self.stream_filtered_chunked(stream_id, 0..=offset, query)
                .map_ok(|chunk| {
                    stream::iter(chunk.data).map(|(_, _, payload)| {
                        payload
                            .extract::<EventRouteMappingEvent>()
                            .map(|event| (event.stream_name, event.stream_nr))
                            .map_err(anyhow::Error::from)
                    })
                })
                .try_flatten()
                .try_collect()
                .await
        } else {
            Ok(HashMap::new())
        }
    }

    /// Creates a new [`BanyanStore`] for testing.
    pub async fn test(node_name: &str) -> Result<Self> {
        Self::new(SwarmConfig::test(node_name), ActoRef::blackhole()).await
    }

    /// Creates a new [`BanyanStore`] for testing.
    pub async fn test_with_routing(node_name: &str, routes: Vec<EventRoute>) -> Result<Self> {
        Self::new(SwarmConfig::test_with_routing(node_name, routes), ActoRef::blackhole()).await
    }

    fn lock(&self) -> BanyanStoreGuard<'_> {
        BanyanStoreGuard {
            data: self.data.clone(),
            state: self.state.clone(),
            guard: self.state.lock(),
        }
    }

    /// Returns the [`NodeId`].
    pub fn node_id(&self) -> NodeId {
        self.data.node_id
    }

    pub fn is_local(&self, stream_id: StreamId) -> bool {
        self.lock().is_local(stream_id)
    }

    /// Returns the underlying [`Ipfs`].
    pub fn ipfs(&self) -> &Ipfs {
        &self.data.ipfs
    }

    /// Resolves a [`Cid`] to a unixfs-v1 [`FileNode`] descriptor. Any needed intermediate blocks
    /// are fetched automatically. The actual data is not resolved.
    pub async fn unixfs_resolve(&self, cid: Cid, name: Option<String>) -> anyhow::Result<FileNode> {
        let peers = self.ipfs().peers();
        let mut tmp = self.ipfs().create_temp_pin()?;
        self.ipfs().temp_pin(&mut tmp, &cid)?;
        let block = self.ipfs().fetch(&cid, peers.clone()).await?;

        match FlatUnixFs::try_parse(block.data()).map_err(|e| anyhow::anyhow!("Error parsing block (: {}", e))? {
            flat if flat.data.Type == UnixFsType::Directory => {
                let mut children = Vec::with_capacity(flat.links.len());
                #[allow(non_snake_case)]
                for PBLink { Hash, Name, Tsize } in flat.links {
                    let cid = Cid::try_from(Hash.as_deref().unwrap_or_default())?;
                    let name = Name.unwrap_or_default().to_string();
                    let size = Tsize.unwrap_or_default();
                    children.push(Child { cid, name, size });
                }
                Ok(FileNode::Directory {
                    children,
                    own_cid: cid,
                    name: name.unwrap_or_else(|| "/".into()),
                })
            }
            file if file.data.Type == UnixFsType::File => Ok(FileNode::File {
                name: name.unwrap_or_default(),
                cid,
            }),
            // Other file types are not supported
            other => {
                anyhow::bail!("Unsupported file type {:?}", other.data.Type);
            }
        }
    }

    /// Resolves a [`Cid`] and a relative path to a unixfs-v1 [`FileNode`] descriptor. Any needed
    /// intermediate blocks are fetched automatically. The actual data is not resolved.
    pub async fn unixfs_resolve_path(&self, cid: Cid, mut path: VecDeque<String>) -> anyhow::Result<FileNode> {
        let mut n = self.unixfs_resolve(cid, None).await?;
        while let Some(segment) = path.pop_front() {
            match n {
                FileNode::Directory { children, own_cid, .. } => {
                    if let Some(x) = children.iter().find(|x| x.name == segment) {
                        n = self.unixfs_resolve(x.cid, Some(segment)).await?;
                    } else {
                        anyhow::bail!("Path {} not found inside {}", segment, own_cid);
                    }
                }
                FileNode::File { name, .. } => {
                    anyhow::bail!("Found file {} while looking for directory {}", name, segment)
                }
            }
        }
        Ok(n)
    }

    /// Traverse a path to a `Cid`. Used for traversing unixfsv1 directories. Make sure you pin
    /// the cid before traversing it.
    pub async fn traverse(&self, cid: &Cid, mut path: VecDeque<String>) -> Result<Option<Cid>> {
        let peers = self.ipfs().peers();
        let mut block = self.ipfs().fetch(cid, peers.clone()).await?;
        let mut cache = None;
        while let Some(segment) = path.pop_front() {
            let mut res = unixfs_v1::dir::resolve(block.data(), segment.as_str(), &mut cache)?;
            loop {
                match res {
                    MaybeResolved::Found(cid) => {
                        block = self.ipfs().fetch(&cid, peers.clone()).await?;
                        break;
                    }
                    MaybeResolved::NotFound => return Ok(None),
                    MaybeResolved::NeedToLoadMore(walker) => {
                        let (cid, _) = walker.pending_links();
                        let block = self.ipfs().fetch(cid, peers.clone()).await?;
                        res = walker.continue_walk(block.data(), &mut cache)?;
                    }
                }
            }
        }
        Ok(Some(block.into_inner().0))
    }

    /// Retrieves the contents of a unixfs-v1 File from the store. If the `pre_sync` bool is set,
    /// the cid will be synced at the beginning. If not, blocks will be fetched on demand.
    pub fn cat(&self, cid: Cid, pre_sync: bool) -> impl Stream<Item = anyhow::Result<Vec<u8>>> {
        stream::try_unfold(
            (self.ipfs().clone(), None, true),
            move |(ipfs, maybe_step, is_first): (Ipfs, Option<FileVisit>, bool)| async move {
                if is_first {
                    debug_assert!(maybe_step.is_none());
                    if pre_sync {
                        ipfs.sync(&cid, ipfs.peers()).await?;
                    }

                    let block = ipfs.fetch(&cid, ipfs.peers()).await?;
                    let (content, _, _, step) = IdleFileVisit::default().start(block.data())?;
                    Ok(Some((content.to_vec(), (ipfs, step, false))))
                } else if let Some(visit) = maybe_step {
                    let (cid, _) = visit.pending_links();
                    let block = ipfs.fetch(cid, ipfs.peers()).await?;
                    let (content, next_step) = visit.continue_walk(block.data(), &mut None)?;

                    Ok(Some((content.to_vec(), (ipfs, next_step, false))))
                } else {
                    Ok(None)
                }
            },
        )
    }

    /// Adds a binary blob to the store. Requires aliasing and flushing before dropping the
    /// `TempPin`.  Blobs are encoded as [unixfs-v1] files.
    ///
    /// [unixfs-v1]: https://docs.ipfs.io/concepts/file-systems/#unix-file-system-unixfs
    pub fn add(&self, tmp: &mut TempPin, reader: impl Read) -> Result<(Cid, usize)> {
        let mut adder = FileAdder::default();
        let mut reader = BufReader::with_capacity(adder.size_hint(), reader);
        let mut bytes_read = 0usize;
        loop {
            match reader.fill_buf()? {
                x if x.is_empty() => {
                    let mut root = None;
                    for (cid, data) in adder.finish() {
                        let block = Block::new_unchecked(cid, data);
                        self.ipfs().temp_pin(tmp, block.cid())?;
                        self.ipfs().insert(block)?;
                        root = Some(cid)
                    }
                    return Ok((root.expect("must return a root"), bytes_read));
                }
                x => {
                    let mut total = 0;
                    while total < x.len() {
                        let (blocks, consumed) = adder.push(&x[total..]);
                        for (cid, data) in blocks {
                            let block = Block::new_unchecked(cid, data);
                            self.ipfs().temp_pin(tmp, block.cid())?;
                            self.ipfs().insert(block)?;
                        }
                        total += consumed;
                    }
                    reader.consume(total);
                    bytes_read += total;
                }
            }
        }
    }

    /// Append events to a stream, publishing the new data.
    pub async fn append(&self, app_id: AppId, events: Vec<(TagSet, Event)>) -> Result<Vec<PersistenceMeta>> {
        let timestamp = Timestamp::now();

        let mut metas = Vec::with_capacity(events.len());
        let mut grouped_events: Vec<(StreamNr, Vec<_>)> = vec![];

        for (tags, payload) in events {
            let stream_nr = self.data.routing_table.get_matching_stream_nr(&tags, &app_id);
            let last_entry = grouped_events.last_mut();
            if let Some((last_stream_nr, events)) = last_entry {
                if *last_stream_nr == stream_nr {
                    events.push((tags, payload));
                    continue;
                }
            }
            // NOTE: should this become a performance bottleneck (probably not now, but I haven’t measured)
            // then this can also be done without allocating additional vectors, by unifying both loops
            // (and taking a slice in append0 instead of the Vec)
            grouped_events.push((stream_nr, vec![(tags, payload)]));
        }

        for (stream_nr, events) in grouped_events {
            let n_events = events.len();
            let append_meta = self.append0(stream_nr, app_id.clone(), timestamp, events).await?;
            metas.extend((0..n_events).map(|n| {
                let n = n as u64;
                (
                    append_meta.min_lamport + n,
                    append_meta.min_offset.increase(n).unwrap(),
                    stream_nr,
                    append_meta.timestamp,
                )
            }));
        }

        Ok(metas)
    }

    async fn append_stream_mapping_event(&self, name: String, number: StreamNr) -> Result<()> {
        let event = EventRouteMappingEvent {
            stream_name: name,
            stream_nr: number,
        };
        let events = vec![(
            ax_types::tags!("event_routing"),
            Event::compact(&event).expect("Should be a valid event."),
        )];
        self.append0(
            DEFAULT_STREAM_NUMBER.into(),
            internal_app_id(),
            Timestamp::now(),
            events,
        )
        .await?;
        Ok(())
    }

    pub async fn append0(
        &self,
        stream_nr: StreamNr,
        app_id: AppId,
        timestamp: Timestamp,
        events: Vec<(TagSet, Event)>,
    ) -> Result<AppendMeta> {
        debug_assert!(!events.is_empty());
        tracing::debug!("publishing {} events on stream {}", events.len(), stream_nr);
        let stream = self.get_or_create_own_stream(stream_nr)?;
        let mut guard = stream.lock().await;

        let _s = tracing::trace_span!("append", stream_nr = display(stream_nr), timestamp = debug(timestamp));
        let _s = _s.enter();

        // We need to keep the store lock to make sure that no other append operations can write
        // to the streams before we are done, because that might break lamport ordering within
        // the streams.
        let mut store = self.lock();
        let mut lamports = store.reserve_lamports(events.len())?.peekable();

        let min_lamport = *lamports.peek().unwrap();
        let app_id_tag = tag!("app_id:") + app_id.as_str();
        let scoped_app_id_tag = ScopedTag::new(crate::trees::tags::TagScope::Internal, app_id_tag);
        let kvs = lamports.zip(events).map(|(lamport, (tags, payload))| {
            let mut tags = ScopedTagSet::from(tags);
            tags.insert(scoped_app_id_tag.clone());
            (AxKey::new(tags, lamport, timestamp), payload)
        });
        let min_offset = self.transform_stream(&mut guard, |txn, tree| {
            let snapshot = tree.snapshot();
            txn.extend_unpacked(tree, kvs)?;
            if tree.level() > MAX_TREE_LEVEL {
                txn.pack(tree)?;
            }
            Ok(snapshot.offset())
        })?;
        let min_offset = min_offset.map(|o| o + 1).unwrap_or(Offset::ZERO);

        Ok(AppendMeta {
            min_lamport,
            min_offset,
            timestamp,
        })
    }

    /// Returns a [`Stream`] of known [`StreamId`].
    pub fn stream_known_streams(&self) -> impl Stream<Item = StreamId> + Send {
        let mut state = self.lock(); // PANIC
        let (s, r) = mpsc::unbounded();
        for stream_id in state.current_stream_ids() {
            let _ = s.unbounded_send(stream_id);
        }
        state.known_streams.push(s);
        r
    }

    /// Returns a [`Stream`] of events filtered with a [`Query`].
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
    ) -> impl Stream<Item = Result<FilteredChunk<(u64, AxKey, Payload), ()>>> {
        tracing::trace!("stream_filtered_chunked {}", stream_id);
        let trees = self.tree_stream(stream_id);
        self.data.forest.stream_trees_chunked(query, trees, range, &|_| {})
    }

    pub fn stream_filtered_chunked_reverse<Q: Query<TT> + Clone + 'static>(
        &self,
        stream_id: StreamId,
        range: RangeInclusive<u64>,
        query: Q,
    ) -> impl Stream<Item = Result<FilteredChunk<(u64, AxKey, Payload), ()>>> {
        let trees = self.tree_stream(stream_id);
        self.data
            .forest
            .stream_trees_chunked_reverse(query, trees, range, &|_| {})
    }

    fn get_or_create_own_stream(&self, stream_nr: StreamNr) -> Result<Arc<OwnStream>> {
        self.lock().get_or_create_own_stream(stream_nr)
    }

    fn get_or_create_replicated_stream(&self, stream_id: StreamId) -> Result<Arc<ReplicatedStream>> {
        self.lock().get_or_create_replicated_stream(stream_id)
    }

    fn transform_stream<T>(
        &self,
        stream: &mut OwnStreamGuard,
        f: impl FnOnce(&mut Transaction, &mut AxStreamBuilder) -> Result<T> + Send,
    ) -> Result<T> {
        let writer = self.data.forest.store().write()?;
        let stream_nr = stream.stream_nr();
        let stream_id = self.node_id().stream(stream_nr);
        let prev = stream.snapshot();
        tracing::debug!("starting write transaction on stream {}", stream_nr);
        let mut txn = Transaction::new(self.data.forest.clone(), writer);
        // take a snapshot of the initial state
        let mut guard = stream.transaction();

        // in case of error: stream builder state will be reverted, except for the cipher offset
        let res = f(&mut txn, &mut guard)?;

        let curr = guard.snapshot();
        if curr.link() == prev.link() {
            // nothing to do, return
            return Ok(res);
        }
        // make sure we did not lose events. If we did, return a failure
        anyhow::ensure!(curr.count() >= prev.count(), "tree rejected because it lost events!");

        // grab the latest lamport
        let lamport = self.data.lamport.get();
        let header = AxTreeHeader::new(curr.link().unwrap(), lamport);
        let root = txn.writer_mut().put(DagCborCodec.encode(&header)?)?;
        let cid = Cid::from(root);
        // update the permanent alias. If this fails, we will revert the builder.
        self.ipfs().alias(StreamAlias::from(stream_id), Some(&cid))?;
        // this concludes the things we want to fail the transaction
        guard.commit();
        // set the latest
        stream
            .latest()
            .set(Some(PublishedTree::new(root, header, curr.clone())));
        // update resent for the stream
        let offset = curr.offset().unwrap();
        self.update_present(stream_id, offset);
        // publish the update - including the header
        let blocks = txn.into_writer().into_written();
        // publish new blocks and root
        self.data.gossip.publish(stream_nr, root, blocks, lamport, offset)?;
        tracing::trace!("transform_stream successful");
        Ok(res)
    }

    fn update_root(&self, stream_id: StreamId, root: Link, source: RootSource) {
        if !self.is_local(stream_id) {
            tracing::trace!("update_root {} {}", stream_id, root);
            self.get_or_create_replicated_stream(stream_id)
                .unwrap()
                .set_incoming(root, source);
        }
    }

    async fn compaction_loop(self, interval: Duration) {
        loop {
            let stream_nrs = self.lock().local_stream_nrs();
            for stream_nr in stream_nrs {
                tracing::debug!("compacting stream {}", stream_nr);
                let stream = self.get_or_create_own_stream(stream_nr).unwrap();
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
    async fn careful_ingestion(self, stream_id: StreamId, state: Arc<ReplicatedStream>) {
        let state2 = state.clone();
        state
            .incoming_root_stream()
            .switch_map(move |(root, source)| {
                self.clone()
                    .sync_one(stream_id, root, source)
                    .map(move |res| (res, root))
                    .into_stream()
            })
            .for_each(|(res, root)| {
                // Must dial down this root’s priority to allow later updates with lower prio.
                // This crucially depends on the fact that sync_one will eventually return, i.e.
                // it must not hang indefinitely. It should ideally fail as quickly as possible
                // when not making progress (but a fixed and short timeout would make it impossible
                // to work on a slow network connection).
                match res {
                    Err(err) => {
                        state2.downgrade(root, true);
                        if let Some(err) = err.downcast_ref::<BlockNotFound>() {
                            tracing::debug!("careful_ingestion: {}", err)
                        } else {
                            tracing::warn!("careful_ingestion: {}", err)
                        }
                    }
                    Ok(outcome) => {
                        tracing::trace!("sync completed {:?}", outcome);
                        state2.downgrade(root, false);
                    }
                }
                future::ready(())
            })
            .await
    }

    /// attempt to sync one stream to a new root.
    ///
    /// this future may be interrupted at any time when an even newer root comes along.
    async fn sync_one(self, stream_id: StreamId, root: Link, source: RootSource) -> Result<SyncOutcome> {
        if source.path == RootPath::SlowPath {
            // it is not unlikely that this sync_one will be replaced by one from the FastPath,
            // so don’t start bitswapping right away
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let s = tracing::trace_span!("sync_one", %stream_id, %root);
        let e = s.enter();

        let cid = Cid::from(root);
        let ipfs = &self.data.ipfs;
        let stream = self.get_or_create_replicated_stream(stream_id)?;
        let (validated_header_lamport, validated_header_count) = stream.validated_tree_counters();
        // temporarily pin the new root
        tracing::trace!("assigning temp pin to {}", root);
        let mut temp_pin = ipfs.create_temp_pin()?;
        ipfs.temp_pin(&mut temp_pin, &cid)?;
        let peers = ipfs.peers();
        // attempt to sync. This may take a while and is likely to be interrupted
        tracing::trace!("starting to sync from {} peers", peers.len());
        // create the sync stream, and log progress. Add an additional element.
        let mut sync = ipfs.sync(&cid, peers).await?;
        // during the sync, try to load the tree asap and abort in case it is not good
        let mut header: Option<AxTreeHeader> = None;
        let mut tree: Option<AxTree> = None;
        let mut n: usize = 0;

        drop(e);

        while let Some(event) = sync.next().await {
            let _e = s.enter();
            match event {
                SyncEvent::Progress { missing } => {
                    tracing::trace!("sync_one: {}/{}", n, n + missing);
                    n += 1;
                }
                SyncEvent::Complete(Err(err)) => {
                    tracing::debug!(%stream_id, %err, "sync_one");
                    return Err(err);
                }
                SyncEvent::Complete(Ok(())) => {}
            }
            if header.is_none() {
                // try to load the header. It should be one of the first things being synced
                if let Ok(blob) = self.data.forest.store().get(&root).surface::<BlockNotFound>()? {
                    let temp: AxTreeHeader = DagCborCodec.decode(&blob)?;
                    if temp.lamport <= validated_header_lamport {
                        // this is not unexpected and should not be logged as an error
                        return Ok(SyncOutcome::OldHeader);
                    }
                    header = Some(temp);
                }
            }
            if let Some(header) = header.as_ref() {
                // try to load the tree. It should come immediately after the header
                if let Ok(temp) = self
                    .data
                    .forest
                    .load_tree(Secrets::default(), header.root)
                    .surface::<BlockNotFound>()?
                {
                    // sanity check: we must never lose events.
                    anyhow::ensure!(temp.count() >= validated_header_count);
                    tree = Some(temp);
                }
            }
        }
        let header = header.ok_or_else(|| anyhow::anyhow!("header was not loaded during sync"))?;
        let tree = tree.ok_or_else(|| anyhow::anyhow!("tree was not loaded during sync"))?;
        let state = PublishedTree::new(root, header, tree.clone());

        // if we get here, we already know that the new tree is better than its predecessor
        tracing::trace!("completed sync of {}", root);
        // once sync is successful, permanently move the alias
        tracing::trace!("updating alias {}", root);
        // assign the new root as validated
        ipfs.alias(StreamAlias::from(stream_id), Some(&cid))?;
        let offset = tree.offset().unwrap();
        tracing::trace!("sync_one complete {} => {}", stream_id, offset);
        stream.set_latest(state);
        // update present.
        self.update_present(stream_id, offset);
        // done
        Ok(SyncOutcome::Success)
    }

    /// Validate that all known streams are completely present
    ///
    /// We could have a lenient mode where this is just logged, or a recovery mode
    /// where it tries to acquire the data on startup, but for now this will just
    /// return an error if anything is missing.
    #[allow(clippy::needless_collect)]
    async fn validate_known_streams(&self) -> Result<()> {
        let state = self.lock();
        let headers = state
            .current_stream_ids()
            .filter_map(|stream_id| state.published_tree(stream_id).map(|p| (stream_id, p.root())))
            .collect::<Vec<_>>();
        drop(state);
        let futures = headers
            .into_iter()
            .map(|(stream_id, root)| async move {
                // sync with 0 peers to just check if we have the data
                let result = self.data.ipfs.sync(&root.into(), vec![]).await;
                (stream_id, result)
            })
            .collect::<Vec<_>>();
        let results = futures::future::join_all(futures).await;
        // log the results
        let mut errors = Vec::new();
        for (stream_id, result) in &results {
            if let Err(cause) = result {
                errors.push(*stream_id);
                tracing::error!("incomplete alias for stream id {}: {}", stream_id, cause);
            } else {
                tracing::debug!("validated alias for stream_id {}", stream_id);
            }
        }
        // fail the entire method in case there is just one failure
        let _ = results
            .into_iter()
            .map(|(_, r)| r)
            .collect::<anyhow::Result<Vec<_>>>()
            .context(format!(
                "Found {} streams with missing events, giving up.",
                errors.len()
            ))?;
        Ok(())
    }

    fn update_present(&self, stream_id: StreamId, offset: Offset) {
        self.data.offsets.transform_mut(|offsets| {
            offsets
                .present
                .update(stream_id, offset)
                .map(|old| tracing::trace!("updating present {} offset {} -> {}", stream_id, old, offset))
                .is_some()
        });
    }

    /// Updates the highest seen for a given stream, if it is higher
    fn update_highest_seen(&self, stream_id: StreamId, offset: Offset) {
        self.data.offsets.transform_mut(|offsets| {
            offsets
                .replication_target
                .update(stream_id, offset)
                .map(|old| tracing::trace!("updating highest {} offset {} -> {}", stream_id, old, offset))
                .is_some()
        });
    }

    fn has_stream(&self, stream_id: StreamId) -> bool {
        self.lock().has_stream(stream_id)
    }

    /// Get a stream of trees for a given stream id
    fn tree_stream(&self, stream_id: StreamId) -> impl Stream<Item = Tree> {
        self.lock().tree_stream(stream_id)
    }

    pub fn spawn_task(&self, name: String, task: BoxFuture<'static, ()>) {
        self.lock().spawn_task(name, task)
    }

    pub fn abort_task(&self, name: &'static str) {
        self.lock().abort_task(name)
    }
}

#[derive(Debug)]
enum SyncOutcome {
    OldHeader,
    Success,
}

trait AnyhowResultExt<T>: Sized {
    /// surface an error out of an anyhow::Error
    fn surface<E: Display + Debug + Send + Sync + 'static>(self) -> anyhow::Result<std::result::Result<T, E>>;
}

impl<T> AnyhowResultExt<T> for anyhow::Result<T> {
    fn surface<E: Display + Debug + Send + Sync + 'static>(self) -> anyhow::Result<std::result::Result<T, E>> {
        match self {
            Ok(result) => Ok(Ok(result)),
            Err(cause) => match cause.downcast::<E>() {
                Ok(cause) => Ok(Err(cause)),
                Err(cause) => Err(cause),
            },
        }
    }
}

trait AxTreeExt {
    fn offset(&self) -> Option<Offset>;
}

impl AxTreeExt for Tree {
    fn offset(&self) -> Option<Offset> {
        match self.count() {
            0 => None,
            n => match Offset::try_from(n - 1) {
                Ok(offset) => Some(offset),
                Err(e) => {
                    panic!("Tree's count ({}) does not fit into an offset. ({})", n, e);
                }
            },
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Child {
    pub name: String,
    #[serde(with = "crate::util::serde_str")]
    pub cid: Cid,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub enum FileNode {
    Directory {
        children: Vec<Child>,
        #[serde(with = "crate::util::serde_str")]
        own_cid: Cid,
        name: String,
    },
    File {
        name: String,
        #[serde(with = "crate::util::serde_str")]
        cid: Cid,
    },
}

/// The "dual" for the event route configuration in the node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRoute {
    pub from: TagExpr,
    pub into: String,
}

impl EventRoute {
    pub fn new(from: TagExpr, into: String) -> Self {
        Self { from, into }
    }

    pub fn discovery() -> Self {
        Self::new(TagExpr::from_str("'discovery'").unwrap(), "discovery".to_string())
    }

    pub fn metrics() -> Self {
        Self::new(TagExpr::from_str("'metrics'").unwrap(), "metrics".to_string())
    }

    pub fn files() -> Self {
        Self::new(
            TagExpr::from_str("'files' | 'files:pinned'").unwrap(),
            "files".to_string(),
        )
    }
}

impl Default for EventRoute {
    fn default() -> Self {
        Self {
            from: TagExpr::from_str("allEvents").expect("Valid tag expression."),
            into: DEFAULT_STREAM_NAME.to_string(),
        }
    }
}

impl FromStr for EventRoute {
    type Err = anyhow::Error;

    /// The expected string has form ["tag_expression", "stream_name"].
    /// This is only expected to be used when parsing command line arguments.
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let tuple: (String, String) = serde_json::from_str(s)?;
        let expr = TagExpr::from_str(&tuple.0)?;
        Ok(Self::new(expr, tuple.1))
    }
}

/// A mapping between a stream's name and its number.
/// Should be extracted as an [Event].
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventRouteMappingEvent {
    stream_name: String,
    stream_nr: StreamNr,
}

trait MatchTagSet {
    fn matches(&self, tag_set: &TagSet, app_id: &AppId) -> bool;
}

impl MatchTagSet for Dnf {
    fn matches(&self, tag_set: &TagSet, app_id: &AppId) -> bool {
        let and_clauses = &self.0;
        for tags in and_clauses {
            let mut matches = true;
            for tag in tags {
                matches &= match tag {
                    TagAtom::Tag(tag) => tag_set.contains(tag),
                    TagAtom::AllEvents => true,
                    TagAtom::AppId(tree_app_id) => tree_app_id == app_id,
                    _ => unreachable!("The validation regex failed."),
                };
            }
            if matches {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod test_match_tag_set {
    use std::str::FromStr;

    use crate::trees::dnf::Dnf;
    use ax_aql::TagExpr;
    use ax_types::{tags, AppId};

    use crate::swarm::{internal_app_id, MatchTagSet};

    #[test]
    fn tag() {
        let dnf = Dnf::from(&TagExpr::from_str("'a'").unwrap());
        assert!(dnf.matches(&tags!("a"), &internal_app_id()));
        assert!(!dnf.matches(&tags!("b"), &internal_app_id()));
    }

    #[test]
    fn or_tags() {
        let dnf = Dnf::from(&TagExpr::from_str("'a' | 'b'").unwrap());
        assert!(dnf.matches(&tags!("b"), &internal_app_id()));
        assert!(!dnf.matches(&tags!("c"), &internal_app_id()));
    }

    #[test]
    fn and_tags() {
        let dnf = Dnf::from(&TagExpr::from_str("'a' & 'b'").unwrap());
        assert!(dnf.matches(&tags!("a", "b"), &internal_app_id()));
        assert!(!dnf.matches(&tags!("c"), &internal_app_id()));
    }

    #[test]
    fn app_id() {
        let dnf = Dnf::from(&TagExpr::from_str("appId(com.actyx)").unwrap());
        assert!(dnf.matches(&tags!(), &internal_app_id()));
        assert!(!dnf.matches(&tags!(), &AppId::from_str("me").unwrap()));
    }

    #[test]
    fn all_events() {
        let dnf = Dnf::from(&TagExpr::from_str("allEvents").unwrap());
        assert!(dnf.matches(&tags!(), &internal_app_id()));
    }

    #[test]
    fn and_all_events() {
        let dnf = Dnf::from(&TagExpr::from_str("'a' & allEvents").unwrap());
        assert!(dnf.matches(&tags!("a"), &internal_app_id()));
        assert!(!dnf.matches(&tags!("b"), &internal_app_id()));
    }
}

#[derive(Default, Debug)]
struct RoutingTable {
    routes: Vec<(Dnf, StreamNr)>,
    stream_mapping: HashMap<String, StreamNr>,
    max_stream_nr: Option<StreamNr>,
}

impl RoutingTable {
    /// Add a new stream to the routing table. Returns the allocated stream number.
    fn add_stream(&mut self, stream: &str, stream_nr: Option<StreamNr>) -> Result<Option<StreamNr>> {
        if let Some(stream_nr) = stream_nr {
            tracing::debug!(
                "Adding stream \"{}\" as #{:?} to the routing table",
                stream,
                u64::from(stream_nr)
            );
            if let Some(nr) = self.stream_mapping.get(stream) {
                tracing::warn!(
                    "Stream {} was previously mapped to {}, replacing with {}.",
                    stream,
                    nr,
                    stream_nr
                );
            }
            self.stream_mapping.insert(stream.to_string(), stream_nr);
            tracing::trace!(
                "Stream \"{}\" was added to the routing table as #{:?}",
                stream,
                u64::from(stream_nr)
            );
            self.max_stream_nr = std::cmp::max(self.max_stream_nr, stream_nr.into());
            Ok(Some(stream_nr))
        } else {
            tracing::trace!("Adding stream \"{}\" to the routing table", stream);
            if self.stream_mapping.get(stream).is_none() {
                self.max_stream_nr = if let Some(stream_nr) = self.max_stream_nr {
                    Some(stream_nr.succ())
                } else {
                    Some(StreamNr::from(0))
                };
                let max_stream_nr = self
                    .max_stream_nr
                    .expect("The maximum stream number can only be None when the structure is initialized.");
                self.stream_mapping.insert(stream.to_string(), max_stream_nr);
                tracing::trace!("Stream \"{}\" was attributed #{:?}", stream, u64::from(max_stream_nr));
                Ok(Some(
                    self.max_stream_nr
                        .expect("The stream number should have already been inserted."),
                ))
            } else {
                Ok(None)
            }
        }
    }

    /// Add a new route to the routing table. Attempts to add the target stream as well.
    /// If a new stream number is allocated returns it.
    ///
    /// Warning: _Does not check for duplicate routes._
    fn add_route(&mut self, matcher: TagExpr, stream: String) -> Option<StreamNr> {
        // Routes should only be added after all existing streams were added to the table,
        // otherwise, stream numbers may be incorrect.
        let stream_nr = self
            .add_stream(&stream, None)
            .expect("Adding a stream without a stream number should never fail.");
        let dnf = Dnf::from(&matcher);

        let nr = if let Some(nr) = stream_nr {
            nr
        } else {
            // This case is triggered when several routes pointing to the same stream are added
            // Such as:
            // match: 'a' & 'b' -> stream_1
            // match: 'a' -> stream_2
            // match: 'b' -> stream_1
            // Even though there are other (and better) ways of expressing this
            *self
                .stream_mapping
                .get(&stream)
                .expect("Stream number must exist here.")
        };
        self.routes.push((dnf, nr));
        stream_nr
    }

    /// "Routes" a provided [TagSet] to the corresponding [StreamNr].
    /// If it is not able to match the [TagSet], it will return `StreamNr::default()`.
    fn get_matching_stream_nr(&self, tag_set: &TagSet, app_id: &AppId) -> StreamNr {
        for (dnf, stream_nr) in self.routes.iter() {
            if dnf.matches(tag_set, app_id) {
                return *stream_nr;
            }
        }
        tracing::trace!("{:?} did not match a stream, sending off to the default", tag_set);
        StreamNr::default()
    }
}

#[cfg(test)]
mod test_routing_table {
    use std::str::FromStr;

    use ax_aql::TagExpr;
    use ax_types::{tags, StreamNr};

    use crate::swarm::{internal_app_id, RoutingTable, DEFAULT_STREAM_NUMBER};

    #[test]
    fn enforce_default_stream_nr_to_be_zero() {
        assert_eq!(StreamNr::default(), StreamNr::from(DEFAULT_STREAM_NUMBER));
    }

    #[test]
    fn empty() {
        assert!(RoutingTable::default().max_stream_nr.is_none());
    }

    #[test]
    fn add_stream() {
        let mut table = RoutingTable::default();
        table.add_stream("stream_1", None).unwrap();
        table.add_stream("stream_2", None).unwrap();
        assert_eq!(*table.stream_mapping.get("stream_1").unwrap(), StreamNr::from(0));
        assert_eq!(*table.stream_mapping.get("stream_2").unwrap(), StreamNr::from(1));
        table.add_stream("stream_2", Some(StreamNr::from(10))).unwrap();
        assert_eq!(*table.stream_mapping.get("stream_2").unwrap(), StreamNr::from(10));
    }

    #[test]
    fn add_route() {
        let mut table = RoutingTable::default();
        table.add_route(TagExpr::from_str("'test'").unwrap(), "stream_1".to_string());
        table.add_route(TagExpr::from_str("'test'").unwrap(), "stream_2".to_string());
        assert_eq!(*table.stream_mapping.get("stream_1").unwrap(), StreamNr::from(0));
        assert_eq!(*table.stream_mapping.get("stream_2").unwrap(), StreamNr::from(1));
        assert_eq!(table.routes[0].1, StreamNr::from(0));
        assert_eq!(table.routes[1].1, StreamNr::from(1));
    }

    #[test]
    fn get_matching_stream_nr() {
        let mut table = RoutingTable::default();
        table.add_stream("default", None).unwrap();
        table.add_route(TagExpr::from_str("'test_1'").unwrap(), "stream_1".to_string());
        table.add_route(TagExpr::from_str("'test_2'").unwrap(), "stream_2".to_string());
        assert_eq!(
            table.get_matching_stream_nr(&tags!("test_1"), &internal_app_id()),
            StreamNr::from(1)
        );
        assert_eq!(
            table.get_matching_stream_nr(&tags!("test_2"), &internal_app_id()),
            StreamNr::from(2)
        );
        assert_eq!(
            table.get_matching_stream_nr(&tags!("non_existing_tag"), &internal_app_id()),
            StreamNr::from(0)
        );
    }

    #[test]
    fn get_matching_stream_nr_with_duplicates() {
        let mut table = RoutingTable::default();
        table.add_route(TagExpr::from_str("'test_0'").unwrap(), "stream_1".to_string());
        table.add_route(TagExpr::from_str("'test_1'").unwrap(), "stream_2".to_string());
        table.add_route(TagExpr::from_str("'test_1'").unwrap(), "stream_3".to_string());
        assert_eq!(
            table.get_matching_stream_nr(&tags!("test_1"), &internal_app_id()),
            StreamNr::from(1)
        );
    }

    #[test]
    fn get_matching_stream_nr_with_expressions() {
        let mut table = RoutingTable::default();
        table.add_route(
            TagExpr::from_str("'test_0' & 'test_1'").unwrap(),
            "stream_0-0_1".to_string(),
        );
        table.add_route(
            TagExpr::from_str("'test_2' ∧ 'test_3'").unwrap(),
            "stream_1-2_3".to_string(),
        );
        table.add_route(
            TagExpr::from_str("'test_4' | 'test_5'").unwrap(),
            "stream_2-4_5".to_string(),
        );
        table.add_route(
            TagExpr::from_str("'test_6' ∨ 'test_7'").unwrap(),
            "stream_3-6_7".to_string(),
        );
        assert_eq!(
            table.get_matching_stream_nr(&tags!("test_0", "test_1"), &internal_app_id()),
            StreamNr::from(0)
        );
        assert_eq!(
            table.get_matching_stream_nr(&tags!("test_2", "test_3"), &internal_app_id()),
            StreamNr::from(1)
        );
        assert_eq!(
            table.get_matching_stream_nr(&tags!("test_4"), &internal_app_id()),
            StreamNr::from(2)
        );
        assert_eq!(
            table.get_matching_stream_nr(&tags!("test_5"), &internal_app_id()),
            StreamNr::from(2)
        );
        assert_eq!(
            table.get_matching_stream_nr(&tags!("test_6"), &internal_app_id()),
            StreamNr::from(3)
        );
        assert_eq!(
            table.get_matching_stream_nr(&tags!("test_7"), &internal_app_id()),
            StreamNr::from(3)
        );
    }
}
