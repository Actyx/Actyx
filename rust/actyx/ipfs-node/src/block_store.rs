//! Interface for the block store as seen from bitswap.
use anyhow::Result;
use futures::{
    channel::mpsc,
    future::{BoxFuture, FutureExt, TryFutureExt},
    Stream,
};
use ipfs_sqlite_block_store::{
    async_block_store::{AsyncBlockStore, RuntimeAdapter},
    cache::{BlockInfo, CacheTracker, SortByIdCacheTracker, WriteInfo},
    SizeTargets, StoreStats,
};
use libipld::Cid;
use libp2p_ax_bitswap::Block;
use parking_lot::Mutex;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tracing::*;

#[derive(Clone)]
struct TokioRuntime(tokio::runtime::Handle);

impl RuntimeAdapter for TokioRuntime {
    fn unblock<F, T>(self, f: F) -> BoxFuture<'static, Result<T>>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.0.spawn_blocking(f).err_into().boxed()
    }

    fn sleep(&self, duration: Duration) -> BoxFuture<()> {
        tokio::time::sleep(duration).boxed()
    }
}

#[derive(Clone)]
pub struct BlockStore(AsyncBlockStore<TokioRuntime>, ChangeStreamProvider);

pub struct BlockAdapter(pub Block);

impl ipfs_sqlite_block_store::Block for BlockAdapter {
    fn cid(&self) -> &Cid {
        self.0.cid()
    }

    fn data(&self) -> &[u8] {
        self.0.data().as_ref()
    }
}

#[derive(Debug)]
struct SendCacheTracker<T>(Arc<Mutex<Vec<mpsc::UnboundedSender<Vec<libipld::Cid>>>>>, T);

impl<T: CacheTracker> SendCacheTracker<T> {
    fn new(inner: T) -> (Self, ChangeStreamProvider) {
        let arc = Arc::new(Mutex::new(Vec::new()));
        (Self(arc.clone(), inner), ChangeStreamProvider(arc))
    }
}

#[derive(Clone)]
struct ChangeStreamProvider(Arc<Mutex<Vec<mpsc::UnboundedSender<Vec<libipld::Cid>>>>>);

impl ChangeStreamProvider {
    /// Stream of blocks added, from the time the method was called.
    ///
    /// Note that this will contain all added cids, even ones that were already in the store
    pub fn blocks_added_stream(&self) -> impl Stream<Item = Vec<libipld::Cid>> {
        let (tx, rx) = mpsc::unbounded();
        self.0.lock().push(tx);
        rx
    }
}

impl<T: CacheTracker> CacheTracker for SendCacheTracker<T> {
    fn blocks_written(&self, blocks: Vec<WriteInfo>) {
        let cids = || blocks.iter().map(|x| *x.cid()).collect();
        self.0.lock().retain(|x| x.unbounded_send(cids()).is_ok());
        self.1.blocks_written(blocks);
    }

    fn blocks_accessed(&self, blocks: Vec<BlockInfo>) {
        self.1.blocks_accessed(blocks);
    }

    fn blocks_deleted(&self, blocks: Vec<BlockInfo>) {
        self.1.blocks_deleted(blocks);
    }

    fn sort_ids(&self, ids: &mut [i64]) {
        self.1.sort_ids(ids);
    }

    fn retain_ids(&self, ids: &[i64]) {
        self.1.retain_ids(ids);
    }
}

impl BlockStore {
    pub fn new(path: Option<PathBuf>, size: u64) -> Result<Self> {
        let rt = tokio::runtime::Handle::try_current()?;
        let (cache_tracker, changes) = SendCacheTracker::new(SortByIdCacheTracker);
        let config = ipfs_sqlite_block_store::Config::default()
            .with_size_targets(SizeTargets::new(100000, size))
            .with_cache_tracker(cache_tracker);
        let store = if let Some(path) = path {
            ipfs_sqlite_block_store::BlockStore::open(path, config)?
        } else {
            ipfs_sqlite_block_store::BlockStore::memory(config)?
        };
        let store = AsyncBlockStore::new(TokioRuntime(rt), store);
        Ok(BlockStore(store, changes))
    }

    pub fn inner(&self) -> &Arc<Mutex<ipfs_sqlite_block_store::BlockStore>> {
        self.0.inner()
    }

    pub fn blocks_added_stream(&self) -> impl Stream<Item = Vec<libipld::Cid>> {
        self.1.blocks_added_stream()
    }

    /// Insert a set of blocks
    pub async fn put_blocks(self, blocks: BTreeSet<Block>) -> Result<()> {
        Ok(self
            .0
            .put_blocks(blocks.into_iter().map(BlockAdapter).collect::<Vec<_>>(), None)
            .await?)
    }

    /// Insert a block
    pub async fn put_block(self, block: Block) -> Result<Cid> {
        let cid = *block.cid();
        self.0.put_blocks(std::iter::once(BlockAdapter(block)), None).await?;
        Ok(cid)
    }

    /// Get a set of blocks
    pub async fn get_blocks(self, cids: impl IntoIterator<Item = Cid> + Send + 'static) -> Result<BTreeSet<Block>> {
        Ok(self
            .0
            .get_blocks(cids)
            .await?
            .filter_map(|(cid, bo)| bo.map(|data| Block::new(data, cid)))
            .collect::<BTreeSet<Block>>())
    }

    /// Get a block
    pub async fn get_block(self, cid: Cid) -> Result<Option<Block>> {
        Ok(self.0.get_block(cid).await?.map(|data| Block::new(data, cid)))
    }

    /// Check if blocks exist
    pub async fn check_blocks(self, cids: BTreeSet<Cid>) -> Result<BTreeMap<Cid, bool>> {
        Ok(self.0.has_blocks(cids).await?)
    }

    /// Remove all blocks that are not pinned
    pub async fn gc(self) -> Result<()> {
        Ok(self.0.gc().await?)
    }

    pub async fn stats(self) -> Result<StoreStats> {
        Ok(self.0.get_store_stats().await?)
    }

    pub async fn alias_many(self, aliases: Vec<(Vec<u8>, Option<Cid>)>) -> Result<()> {
        Ok(self.0.alias_many(aliases).await?)
    }

    pub async fn get_missing_blocks(self, cid: Cid) -> Result<BTreeSet<Cid>> {
        debug!("get_missing_blocks {}", cid);
        let missing = self.0.get_missing_blocks::<BTreeSet<_>>(cid).await?;
        debug!(
            "get_missing_blocks result {}",
            missing.iter().map(|cid| cid.to_string()).collect::<Vec<_>>().join(",")
        );
        Ok(missing)
    }
}
