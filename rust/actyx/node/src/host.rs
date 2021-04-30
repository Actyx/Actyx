use crate::{node_storage::NodeStorage, util::make_keystore};
use actyxos_sdk::NodeId;
use crypto::KeyStoreRef;
use parking_lot::Mutex;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use util::formats::NodeCycleCount;

pub(crate) struct Host {
    base_path: PathBuf,
    keystore: KeyStoreRef,
    storage: NodeStorage,
}
impl Host {
    pub fn new(base_path: PathBuf) -> Self {
        #[cfg(test)]
        let storage = NodeStorage::in_memory();
        #[cfg(not(test))]
        let storage = NodeStorage::new(base_path.join("node.sqlite")).expect("Error creating node.sqlite");
        let keystore = make_keystore(storage.clone()).unwrap();
        Self {
            base_path,
            keystore,
            storage,
        }
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
