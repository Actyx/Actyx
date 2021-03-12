use crate::behaviour::Behaviour;
use fnv::FnvHashMap;
use futures::{channel::mpsc, Stream};
use libipld::Cid;
use libp2p::Swarm;
use parking_lot::Mutex;
use std::{
    collections::BTreeSet,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

#[derive(Debug, Clone, Copy)]
pub enum SyncProgress {
    BlocksReceived(usize),
    MissingBlocksFound(usize),
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub(crate) struct SyncId(pub u64);

/// this is basically a wrapper around an unbounded stream that performs an action on drop.
pub struct SyncStream {
    swarm: Arc<Mutex<Swarm<Behaviour>>>,
    id: SyncId,
    rx: mpsc::UnboundedReceiver<anyhow::Result<SyncProgress>>,
}

impl SyncStream {
    pub(crate) fn new(
        swarm: Arc<Mutex<Swarm<Behaviour>>>,
        id: SyncId,
        rx: mpsc::UnboundedReceiver<anyhow::Result<SyncProgress>>,
    ) -> Self {
        Self { swarm, id, rx }
    }
}

impl Drop for SyncStream {
    fn drop(&mut self) {
        self.swarm.lock().sync_states.stop_sync(self.id);
    }
}

impl Stream for SyncStream {
    type Item = anyhow::Result<SyncProgress>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.rx).poll_next(cx)
    }
}

pub(crate) struct Syncer {
    /// root being synced
    pub root: Cid,
    /// set of missing cids.
    /// This will be empty initially, so this being empty does not mean that the sync is complete!
    pub missing: BTreeSet<Cid>,
    /// sender for completing the future with success or failure
    pub tx: mpsc::UnboundedSender<anyhow::Result<SyncProgress>>,
}

impl Syncer {
    pub fn new(root: Cid, tx: mpsc::UnboundedSender<anyhow::Result<SyncProgress>>) -> Self {
        Self {
            root,
            tx,
            missing: BTreeSet::new(),
        }
    }

    pub fn send_abort(self, cause: anyhow::Error) {
        let _ = self.tx.unbounded_send(Err(cause));
    }

    pub fn send_progress(&mut self, progress: SyncProgress) {
        let _ = self.tx.unbounded_send(Ok(progress));
    }
}

pub(crate) struct SyncStates {
    pub current: FnvHashMap<SyncId, Syncer>,
    pub next_id: u64,
}

impl SyncStates {
    pub fn new() -> Self {
        Self {
            current: FnvHashMap::default(),
            next_id: 0,
        }
    }

    /// all cids that are currently being synced, for the bitswap want cleanup
    pub fn cids(&self) -> impl Iterator<Item = Cid> + '_ {
        self.current.values().map(|syncer| &syncer.missing).flatten().cloned()
    }

    fn stop_sync(&mut self, sync_id: SyncId) {
        self.current.remove(&sync_id);
    }
}
