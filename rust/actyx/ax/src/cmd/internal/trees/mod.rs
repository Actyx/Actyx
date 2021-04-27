mod dump;
mod explore;

use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;
use trees::axtrees::Sha256Digest;
use TreesOpts::DumpTree;

use self::dump::DumpTreeOpts;
use self::explore::ExploreTreeOpts;

#[derive(StructOpt, Debug)]
/// Manage ActyxOS nodes
pub enum TreesOpts {
    #[structopt(name = "dump")]
    /// Dump contents of banyan trees stored in sqlite. Works with either a
    /// single tree or a data blob.
    DumpTree(DumpTreeOpts),
    #[structopt(name = "explore")]
    ExploreTree(ExploreTreeOpts),
}

pub fn run(opts: TreesOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        DumpTree(opts) => dump::DumpTree::output(opts, json),
        TreesOpts::ExploreTree(opts) => explore::ExploreTree::output(opts, json),
    }
}

use banyan::store::{BlockWriter, ReadOnlyStore};
use ipfs_sqlite_block_store::BlockStore;
use libipld::{Block, Cid, DefaultParams};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) struct SqliteStore(Arc<Mutex<BlockStore<DefaultParams>>>);

impl SqliteStore {
    pub fn new(store: BlockStore<DefaultParams>) -> anyhow::Result<Self> {
        Ok(SqliteStore(Arc::new(Mutex::new(store))))
    }
}

impl ReadOnlyStore<Sha256Digest> for SqliteStore {
    fn get(&self, link: &Sha256Digest) -> anyhow::Result<Box<[u8]>> {
        let cid = Cid::from(*link);
        let block = self.0.lock().unwrap().get_block(&cid)?;
        if let Some(block) = block {
            Ok(block.into())
        } else {
            Err(anyhow::anyhow!("block not found!"))
        }
    }
}

impl BlockWriter<Sha256Digest> for SqliteStore {
    fn put(&self, data: Vec<u8>) -> anyhow::Result<Sha256Digest> {
        let digest = Sha256Digest::new(&data);
        let cid = digest.into();
        let block = Block::new_unchecked(cid, data);
        self.0.lock().unwrap().put_block(&block, None)?;
        Ok(digest)
    }
}
