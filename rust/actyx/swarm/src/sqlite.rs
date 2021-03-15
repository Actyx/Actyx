//! helper methods to work with ipfs/ipld
use crate::{Block, Ipfs};
use anyhow::Result;
use banyan::store::{BlockWriter, ReadOnlyStore};
use core::fmt;
use ipfs_embed::TempPin;
use libipld::Cid;
use parking_lot::Mutex;
use std::collections::BTreeSet;
use trees::axtrees::Sha256Digest;

#[derive(Clone)]
pub struct SqliteStore(Ipfs);

impl fmt::Debug for SqliteStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqliteStore").finish()
    }
}

impl SqliteStore {
    pub fn wrap(inner: Ipfs) -> Self {
        SqliteStore(inner)
    }

    pub fn write(&self) -> Result<SqliteStoreWrite> {
        let store = self.clone();
        let pin = self.0.create_temp_pin()?;
        Ok(SqliteStoreWrite {
            store,
            pin,
            written: Mutex::new(BTreeSet::new()),
        })
    }
}

impl ReadOnlyStore<Sha256Digest> for SqliteStore {
    fn get(&self, link: &Sha256Digest) -> Result<Box<[u8]>> {
        let cid = Cid::from(*link);
        let block = self.0.get(&cid)?;
        let (_cid, data) = block.into_inner();
        Ok(data.into_boxed_slice())
    }
}

pub struct SqliteStoreWrite {
    store: SqliteStore,
    pin: TempPin,
    written: Mutex<BTreeSet<Sha256Digest>>,
}

impl SqliteStoreWrite {
    pub fn into_written(self) -> BTreeSet<Sha256Digest> {
        self.written.into_inner()
    }
}

impl BlockWriter<Sha256Digest> for SqliteStoreWrite {
    fn put(&self, data: Vec<u8>) -> Result<Sha256Digest> {
        let digest = Sha256Digest::new(&data);
        let cid = digest.into();
        let block = Block::new_unchecked(cid, data);
        self.store.0.temp_pin(&self.pin, &cid)?;
        let _ = self.store.0.insert(&block)?;
        self.written.lock().insert(digest);
        Ok(digest)
    }
}
