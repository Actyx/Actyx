pub mod access;
mod connectivity;
pub mod convert;
mod discovery;
pub mod metrics;
mod node_identity;
mod sqlite;
mod sqlite_index_store;
mod streams;
pub mod transport;
mod unixfsv1;

#[cfg(test)]
mod tests;
mod v1;
mod v2;

pub use crate::connectivity::{Connectivity, ConnectivityCalculator};
pub use crate::node_identity::NodeIdentity;
pub use crate::sqlite_index_store::DbPath;
pub use crate::v1::{EventStore, HighestSeen, Present, SnapshotStore};
pub use ax_config::StoreConfig;

use crate::connectivity::ConnectivityState;
use crate::sqlite::{SqliteStore, SqliteStoreWrite};
use crate::sqlite_index_store::SqliteIndexStore;
use crate::streams::{OwnStreamInner, ReplicatedStreamInner, StreamAlias, StreamMaps};
use actyxos_sdk::{
    source_id,
    tagged::{NodeId, StreamId, StreamNr, TagSet},
    LamportTimestamp, Offset, Payload, TimeStamp,
};
use anyhow::Result;
use ax_futures_util::{prelude::*, stream::variable::Variable};
use banyan::{
    forest::{self, BranchCache},
    index::Index,
    query::Query,
};
use forest::FilteredChunk;
use futures::{channel::mpsc, prelude::*};
use ipfs_embed::{BitswapConfig, Config as IpfsConfig, NetworkConfig, StorageConfig, SyncEvent};
use libipld::Cid;
use libp2p::{
    gossipsub::{GossipsubConfigBuilder, ValidationMode},
    multiaddr::Protocol,
    ping::PingConfig,
    pnet::PreSharedKey,
    PeerId,
};
use parking_lot::Mutex;
use std::{
    collections::VecDeque, fmt::Debug, num::NonZeroU32, ops::RangeInclusive, str::FromStr, sync::Arc, time::Duration,
};
use trees::{
    axtrees::{AxKey, AxTrees, Sha256Digest},
    OffsetMapOrMax,
};

type TT = AxTrees;
type Key = AxKey;
type Event = Payload;
type Forest = banyan::forest::Forest<TT, Event, SqliteStore>;
type Transaction = banyan::forest::Transaction<TT, Event, SqliteStore, SqliteStoreWrite>;
type Link = Sha256Digest;
type Tree = banyan::tree::Tree<TT>;

pub type Block = libipld::Block<libipld::DefaultParams>;
pub type Ipfs = ipfs_embed::Ipfs<libipld::DefaultParams>;

#[derive(Debug, Clone)]
pub struct Config {
    branch_cache: usize,
    crypto_config: forest::CryptoConfig,
    forest_config: forest::Config,
    topic: String,
    node_id: NodeId,
}

impl Config {
    pub fn new(topic: &str, node_id: NodeId) -> Self {
        Self {
            branch_cache: 1000,
            crypto_config: Default::default(),
            forest_config: forest::Config::debug(),
            topic: topic.into(),
            node_id,
        }
    }

    pub fn test() -> Self {
        Self {
            branch_cache: 1000,
            crypto_config: Default::default(),
            forest_config: forest::Config::debug(),
            topic: "test".into(),
            node_id: source_id!("test").into(),
        }
    }
}

/// Stream manager.
#[derive(Clone)]
pub struct BanyanStore(Arc<BanyanStoreInner>);

/// internal state of the stream manager
struct BanyanStoreInner {
    maps: Mutex<StreamMaps>,
    gossip_v2: v2::GossipV2,
    forest: Forest,
    ipfs: Ipfs,
    node_id: NodeId,
    index_store: Mutex<SqliteIndexStore>,
    /// maximum ingested offset for each source (later: each stream)
    present: Variable<OffsetMapOrMax>,
    /// highest seen offset for each source (later: each stream)
    highest_seen: Variable<OffsetMapOrMax>,
    /// lamport timestamp for publishing to internal streams
    lamport: Variable<LamportTimestamp>,
    /// fields related to the connectivity mechanism
    connectivity: ConnectivityState,
    /// tasks of the stream manager.
    tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl Drop for BanyanStoreInner {
    fn drop(&mut self) {
        for task in self.tasks.lock().drain(..) {
            task.abort();
        }
    }
}

impl BanyanStore {
    /// Creates a new [`BanyanStore`] from a [`StoreConfig`].
    pub async fn from_axconfig(cfg: ax_config::StoreConfig) -> Result<Self> {
        Self::from_axconfig0(cfg, None).await
    }

    /// Creates a new [`BanyanStore`] from a [`StoreConfig`].
    /// Irrespective of what's configured in [`StoreConfig`], the provided
    /// [`rusqlite::Connection`] will be used for the index store.
    pub async fn from_axconfig_with_db(
        cfg: ax_config::StoreConfig,
        db: Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<Self> {
        Self::from_axconfig0(cfg, Some(db)).await
    }

    async fn from_axconfig0(cfg: ax_config::StoreConfig, db: Option<Arc<Mutex<rusqlite::Connection>>>) -> Result<Self> {
        tracing::debug!("client_from_config({:?})", cfg);
        tracing::debug!("Starting up in IPFS full node mode");
        if cfg.ipfs_node.enable_publish {
            tracing::debug!("Publishing is allowed to pubsub");
        } else {
            tracing::info!("Publishing is disabled to pubsub");
        }

        let identity = if let Some(identity) = cfg.ipfs_node.identity {
            NodeIdentity::from_str(&identity)?
        } else {
            NodeIdentity::generate()
        };

        let config = IpfsConfig {
            network: NetworkConfig {
                node_key: identity.to_keypair(),
                node_name: names::Generator::with_naming(names::Name::Numbered).next().unwrap(),
                enable_mdns: true,
                enable_kad: false,
                allow_non_globals_in_dht: false,
                psk: if let Some(psk) = cfg.ipfs_node.pre_shared_key {
                    let blob = base64::decode(psk)?;
                    let decoded = String::from_utf8(blob)?;
                    Some(PreSharedKey::from_str(&decoded)?)
                } else {
                    None
                },
                ping: PingConfig::new()
                    .with_keep_alive(true)
                    .with_max_failures(NonZeroU32::new(2).unwrap()),
                gossipsub: GossipsubConfigBuilder::default()
                    .validation_mode(ValidationMode::Permissive)
                    .build()
                    .expect("valid gossipsub config"),
                bitswap: BitswapConfig {
                    request_timeout: Duration::from_secs(10),
                    connection_keep_alive: Duration::from_secs(10),
                },
            },
            storage: StorageConfig {
                path: cfg.ipfs_node.db_path,
                cache_size_blocks: u64::MAX,
                cache_size_bytes: cfg.ipfs_node.db_size.unwrap_or(1024 * 1024 * 1024 * 4),
                gc_interval: Duration::from_secs(10),
                gc_min_blocks: 1000,
                gc_target_duration: Duration::from_millis(10),
            },
        };
        let ipfs = Ipfs::new(config).await?;
        for addr in cfg.ipfs_node.listen {
            let bound_addr = ipfs.listen_on(addr).await?;
            tracing::info!(target: "SWARM_SERVICES_BOUND", "Swarm Services bound to {}.", bound_addr);
        }
        for addr in cfg.ipfs_node.external_addresses {
            ipfs.add_external_address(addr);
        }
        for mut addr in cfg.ipfs_node.bootstrap {
            if let Some(Protocol::P2p(peer_id)) = addr.pop() {
                let peer_id =
                    PeerId::from_multihash(peer_id).map_err(|_| anyhow::anyhow!("invalid bootstrap peer id"))?;
                ipfs.dial_address(&peer_id, addr)?;
            } else {
                return Err(anyhow::anyhow!("invalid bootstrap address"));
            }
        }

        let config = Config::new(&cfg.topic, identity.into());
        let index_store = if let Some(con) = db {
            SqliteIndexStore::from_conn(con)?
        } else {
            let db_path = if let Some(path) = cfg.db_path {
                tracing::debug!("Initializing SQLite index store: {}", path.display());
                DbPath::File(path)
            } else {
                tracing::debug!("Initializing SQLite index store: IN MEMORY");
                DbPath::Memory
            };
            SqliteIndexStore::open(db_path)?
        };
        let banyan = BanyanStore::new(ipfs, config, index_store)?;
        tracing::debug!(
            "Start listening on topic '{}' using monitoring topic '{}'",
            &cfg.topic,
            &cfg.monitoring_topic,
        );
        Ok(banyan)
    }

    /// Creates  a new [`BanyanStore`] from an [`Ipfs`] and [`Config`].
    pub fn new(ipfs: Ipfs, config: Config, index_store: SqliteIndexStore) -> Result<Self> {
        let store = SqliteStore::wrap(ipfs.clone());
        let branch_cache = BranchCache::<TT>::new(config.branch_cache);
        let connectivity = ConnectivityState::new();
        let node_id = config.node_id;
        let index_store = Mutex::new(index_store);
        let gossip_v2 = v2::GossipV2::new(ipfs.clone(), node_id, config.topic.clone());
        let me = Self(Arc::new(BanyanStoreInner {
            maps: Mutex::new(StreamMaps::default()),
            index_store,
            node_id,
            ipfs: ipfs.clone(),
            gossip_v2,
            lamport: Default::default(),
            present: Default::default(),
            highest_seen: Default::default(),
            forest: Forest::new(store, branch_cache, config.crypto_config, config.forest_config),
            connectivity,
            tasks: Default::default(),
        }));
        me.spawn_task(
            "v2_gossip_ingest",
            me.0.gossip_v2.ingest(me.clone(), config.topic.clone())?,
        );
        // TODO: me.load for own streams
        me.spawn_task("compaction", me.clone().compaction_loop(Duration::from_secs(60)));
        me.spawn_task("v1_gossip_publish", me.clone().v1_gossip_publish(config.topic.clone()));
        me.spawn_task("v1_gossip_ingest", me.clone().v1_gossip_ingest(config.topic));
        me.spawn_task("discovery_ingest", crate::discovery::discovery_ingest(ipfs.clone())?);
        me.spawn_task("discovery_publish", crate::discovery::discovery_publish(ipfs));
        // TODO fix stream nr
        me.spawn_task(
            "metrics",
            crate::metrics::metrics(me.clone(), 42.into(), Duration::from_secs(30))?,
        );
        Ok(me)
    }

    /// Returns the [`NodeId`].
    pub fn node_id(&self) -> NodeId {
        self.0.node_id
    }

    /// Returns the underlying [`Ipfs`].
    pub fn ipfs(&self) -> &Ipfs {
        &self.0.ipfs
    }

    pub fn cat(&self, cid: Cid, path: VecDeque<String>) -> impl Stream<Item = Result<Vec<u8>>> + Send {
        unixfsv1::UnixfsStream::new(unixfsv1::UnixfsDecoder::new(self.ipfs().clone(), cid, path))
    }

    /// Append events to a stream, publishing the new data.
    pub async fn append(&self, stream_nr: StreamNr, events: Vec<(TagSet, Event)>) -> Result<Option<Link>> {
        tracing::info!("publishing {} events on stream {}", events.len(), stream_nr);
        let lamport = self.0.index_store.lock().increment_lamport()?;
        let timestamp = TimeStamp::now();
        let events = events
            .into_iter()
            .map(move |(tags, event)| (Key::new(tags, lamport, timestamp), event));
        self.transform_stream(stream_nr, |txn, tree| txn.extend_unpacked(tree, events))
            .await
    }

    /// Spawns a new task that will be shutdown when [`BanyanStore`] is dropped.
    pub fn spawn_task(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) {
        tracing::debug!("Spawning task '{}'!", name);
        let handle =
            tokio::spawn(task.map(move |_| tracing::error!("Fatal: Task '{}' unexpectedly terminated!", name)));
        self.0.tasks.lock().push(handle);
    }

    /// Returns a [`Stream`] of known [`StreamId`].
    pub fn stream_known_streams(&self) -> impl Stream<Item = StreamId> + Send {
        let mut state = self.0.maps.lock();
        let (s, r) = mpsc::unbounded();
        for stream_id in state.current_stream_ids(self.0.node_id) {
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
        tracing::info!("stream_filtered_chunked {}", stream_id);
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

    /// careful ingestion - basically just call sync_one on each new ingested root
    async fn careful_ingestion(self, stream_id: StreamId, state: Arc<ReplicatedStreamInner>) {
        state
            .incoming_root_stream()
            .switch_map(move |root| self.clone().sync_one(stream_id, root).into_stream())
            .for_each(|_| future::ready(()))
            .await
    }

    fn get_or_create_own_stream(&self, stream_nr: StreamNr) -> Arc<OwnStreamInner> {
        let mut maps = self.0.maps.lock();
        maps.own_streams.get(&stream_nr).cloned().unwrap_or_else(|| {
            tracing::info!("creating new own stream {}", stream_nr);
            let forest = self.0.forest.clone();
            let stream_id = self.node_id().stream(stream_nr);
            // TODO: Maybe this fn should be fallible
            let _ = self.0.index_store.lock().add_stream(stream_id);
            tracing::info!("publish new stream_id {}", stream_id);
            maps.publish_new_stream_id(stream_id);
            let stream = Arc::new(OwnStreamInner::new(forest));
            maps.own_streams.insert(stream_nr, stream.clone());
            stream
        })
    }

    fn get_or_create_replicated_stream(&self, stream_id: StreamId) -> Arc<ReplicatedStreamInner> {
        assert!(self.node_id() != stream_id.node_id());
        let mut maps = self.0.maps.lock();
        let node_id = stream_id.node_id();
        let stream_nr = stream_id.stream_nr();
        let remote_node = maps.get_or_create_remote_node(node_id);
        if let Some(state) = remote_node.streams.get(&stream_nr).cloned() {
            state
        } else {
            tracing::info!("creating new replicated stream {}", stream_id);
            let forest = self.0.forest.clone();
            let state = Arc::new(ReplicatedStreamInner::new(forest));
            self.spawn_task(
                "careful_ingestion",
                self.clone().careful_ingestion(stream_id, state.clone()),
            );
            remote_node.streams.insert(stream_nr, state.clone());
            tracing::info!("publish new stream_id {}", stream_id);
            maps.publish_new_stream_id(stream_id);
            state
        }
    }

    fn transform_stream(
        &self,
        stream_nr: StreamNr,
        f: impl FnOnce(&Transaction, &Tree) -> Result<Tree> + Send,
    ) -> impl Future<Output = Result<Option<Link>>> {
        let this = self.clone();
        async move {
            let stream = this.get_or_create_own_stream(stream_nr);
            let lock = stream.sequencer.lock().await;
            let writer = this.0.forest.store().write()?;
            tracing::debug!("starting write transaction on stream {}", stream_nr);
            let txn = Transaction::new(stream.forest.clone(), writer);
            let curr = stream.latest();
            let tree = f(&txn, &curr)?;
            // root of the new tree
            let root: Option<Link> = tree.link();
            // check for change
            if root != curr.link() {
                let cid: Option<Cid> = root.map(Into::into);
                let stream_id = this.node_id().stream(stream_nr);
                tracing::debug!(
                    "updating alias for stream {} to {:?}",
                    stream_nr,
                    cid.map(|x: Cid| x.to_string())
                );
                // update the permanent alias
                this.ipfs().alias(StreamAlias::from(stream_id), cid.as_ref())?;
                // update latest
                tracing::debug!("set_latest! {}", tree);
                stream.set_latest(tree);
                let blocks = txn.into_writer().into_written();
                // publish new blocks and root
                this.0.gossip_v2.publish(stream_nr, root.expect("not None"), blocks)?;
            }
            tracing::debug!("ended write transaction on stream {}", stream_nr);
            drop(lock);
            Ok(root)
        }
    }

    fn update_root(&self, stream_id: StreamId, root: Link) {
        tracing::info!("update_root {} {}", stream_id, root);
        self.get_or_create_replicated_stream(stream_id).set_incoming(root);
    }

    async fn compaction_loop(self, interval: Duration) {
        loop {
            if let Err(err) = self.compact_once().await {
                tracing::error!("{}", err);
            }
            tokio::time::sleep(interval).await;
        }
    }

    async fn compact_once(&self) -> Result<()> {
        let stream_ids = self.0.maps.lock().own_streams.keys().cloned().collect::<Vec<_>>();
        for stream_id in stream_ids {
            tracing::info!("compacting stream {}", stream_id);
            self.pack(stream_id).await?;
        }
        Ok(())
    }

    fn pack(&self, stream_nr: StreamNr) -> impl Future<Output = Result<Option<Link>>> {
        tracing::info!("packing stream {}", stream_nr);
        self.transform_stream(stream_nr, |txn, tree| txn.pack(tree))
    }

    /// attempt to sync one stream to a new root.
    ///
    /// this future may be interrupted at any time when an even newer root comes along.
    async fn sync_one(self, stream_id: StreamId, root: Link) -> Result<()> {
        // tokio::time::delay_for(Duration::from_millis(10)).await;
        tracing::debug!("starting to sync {} to {}", stream_id, root);
        let cid = Cid::from(root);
        let ipfs = &self.0.ipfs;
        let stream = self.get_or_create_replicated_stream(stream_id);
        let validated_lamport = stream.validated().last_lamport();
        // temporarily pin the new root
        tracing::debug!("assigning temp pin to {}", root);
        let temp_pin = ipfs.create_temp_pin()?;
        ipfs.temp_pin(&temp_pin, &cid)?;
        // attempt to sync. This may take a while and is likely to be interrupted
        tracing::debug!("starting to sync {}", root);
        // create the sync stream, and log progress. Add an additional element.
        let mut sync = ipfs.sync(&cid);
        // during the sync, try to load the tree asap and abort in case it is not good
        let mut tree: Option<Tree> = None;
        let mut n: usize = 0;
        while let Some(event) = sync.next().await {
            match event {
                SyncEvent::Progress { missing } => {
                    tracing::info!("sync_one: {}/{}", n, n + missing);
                    n += 1;
                }
                SyncEvent::Complete(Err(err)) => return Err(err),
                SyncEvent::Complete(Ok(())) => {}
            }
            if tree.is_none() {
                // load the tree as soon as possible. If this fails, bail out.
                let temp = stream.forest.load_tree(root)?;
                // check that the tree is better than the one we have, otherwise bail out
                anyhow::ensure!(temp.last_lamport() > validated_lamport);
                // get the offset
                let offset = temp.count();
                // update present. This can fail if stream_id is not a source_id.
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
        self.0.index_store.lock().received_lamport(tree.last_lamport().into())?;
        tracing::debug!("sync_one complete {} => {}", stream_id, tree.count());
        let offset = tree.count();
        stream.set_latest(tree);
        // update present. This can fail if stream_id is not a source_id.
        let _ = self.update_present(stream_id, offset);
        // done
        Ok(())
    }

    fn has_stream(&self, stream_id: StreamId) -> bool {
        let me = stream_id.node_id() == self.node_id();
        let maps = self.0.maps.lock();
        if me {
            maps.own_streams.contains_key(&stream_id.stream_nr())
        } else {
            maps.remote_nodes
                .get(&stream_id.node_id())
                .map(|node| node.streams.contains_key(&stream_id.stream_nr()))
                .unwrap_or_default()
        }
    }

    /// stream of latest updates from either gossip (for replicated streams) or internal updates
    ///
    /// note that this does not include event updates
    fn latest_stream(&self, stream_id: StreamId) -> impl Stream<Item = (LamportTimestamp, Offset)> {
        if stream_id.node_id() == self.node_id() {
            let stream = self.get_or_create_own_stream(stream_id.stream_nr());
            self.0
                .lamport
                .new_observer()
                .filter_map(move |lamport| future::ready(stream.offset().map(|offset| (lamport, offset))))
                .left_stream()
        } else {
            self.get_or_create_replicated_stream(stream_id)
                .latest_seen
                .new_observer()
                .filter_map(future::ready)
                .right_stream()
        }
    }

    /// Get a stream of trees for a given stream id
    fn tree_stream(&self, stream_id: StreamId) -> (impl Stream<Item = Tree>, Forest) {
        let me = stream_id.node_id() == self.node_id();
        if me {
            let stream_nr = stream_id.stream_nr();
            let stream = self.get_or_create_own_stream(stream_nr);
            (stream.tree_stream(), stream.forest.clone())
        } else {
            let stream = self.get_or_create_replicated_stream(stream_id);
            (stream.tree_stream(), stream.forest.clone())
        }
    }
}

trait AxTreeExt {
    fn last_lamport(&self) -> LamportTimestamp;
}

impl AxTreeExt for Tree {
    fn last_lamport(&self) -> LamportTimestamp {
        match self.as_index_ref() {
            Some(Index::Branch(branch)) => branch.summaries.lamport_range().max,
            Some(Index::Leaf(leaf)) => leaf.keys.lamport_range().max,
            None => Default::default(),
        }
    }
}
