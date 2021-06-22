use crate::{node_storage::NodeStorage, util::make_keystore};
use actyx_sdk::NodeId;
use anyhow::{Context, Result};
use crypto::KeyStoreRef;
use derive_more::Display;
#[cfg(not(target_os = "android"))]
use fslock::LockFile;
use parking_lot::Mutex;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use util::formats::NodeCycleCount;

#[derive(Debug, Clone, Display)]
#[display(fmt = "data directory `{}` is locked by another Actyx process", _0)]
pub struct WorkdirLocked(String);
impl std::error::Error for WorkdirLocked {}

pub(crate) struct Host {
    base_path: PathBuf,
    keystore: KeyStoreRef,
    storage: NodeStorage,
    #[allow(dead_code)] // this needs to be kept around to hold the lock
    #[cfg(not(target_os = "android"))]
    lockfile: LockFile,
}
impl Host {
    pub fn new(base_path: PathBuf) -> Result<Self> {
        #[cfg(not(target_os = "android"))]
        let lockfile = {
            let mut lf = LockFile::open(&base_path.join("lockfile")).context("Error opening file `lockfile`")?;
            if !lf.try_lock().context("Error locking file `lockfile`")? {
                return Err(WorkdirLocked(base_path.display().to_string()).into());
            }
            lf
        };

        #[cfg(test)]
        let storage = NodeStorage::in_memory();
        #[cfg(not(test))]
        let storage = NodeStorage::new(base_path.join("node.sqlite")).context("Error opening node.sqlite")?;

        let keystore = make_keystore(storage.clone())?;

        Ok(Self {
            base_path,
            keystore,
            storage,
            #[cfg(not(target_os = "android"))]
            lockfile,
        })
    }

    pub fn working_dir(&self) -> &Path {
        &self.base_path
    }

    /// Returns this node's NodeId
    pub fn get_or_create_node_id(&self) -> anyhow::Result<NodeId> {
        if let Some(key_id) = self.storage.get_node_key()? {
            Ok(key_id)
        } else {
            let node_id: NodeId = self.keystore.write().generate_key_pair()?.into();
            self.storage.set_node_id(node_id)?;
            Ok(node_id)
        }
    }

    pub fn get_keystore(&self) -> KeyStoreRef {
        self.keystore.clone()
    }

    pub fn get_db_handle(&self) -> Arc<Mutex<rusqlite::Connection>> {
        self.storage.connection.clone()
    }

    pub fn get_cycle_count(&self) -> anyhow::Result<NodeCycleCount> {
        self.storage.get_cycle_count()
    }
}
