use crate::{crypto_cell::CryptoCell, node_storage::NodeStorage, util::make_keystore};
use crypto::KeyStoreRef;
use parking_lot::Mutex;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub(crate) struct Host {
    base_path: PathBuf,
    keystore: KeyStoreRef,
    crypto_cell: CryptoCell,
    storage: NodeStorage,
}
impl Host {
    pub fn new(base_path: PathBuf) -> Self {
        #[cfg(test)]
        let storage = NodeStorage::in_memory();
        #[cfg(not(test))]
        let storage = NodeStorage::new(base_path.join("node.sqlite")).expect("Error creating node.sqlite");
        let keystore = make_keystore(storage.clone()).unwrap();
        let crypto_cell = CryptoCell::new(keystore.clone(), storage.clone());
        Self {
            base_path,
            keystore,
            crypto_cell,
            storage,
        }
    }

    pub fn working_dir(&self) -> &Path {
        &self.base_path
    }

    pub fn get_crypto_cell(&self) -> &CryptoCell {
        &self.crypto_cell
    }

    pub fn get_keystore(&self) -> KeyStoreRef {
        self.keystore.clone()
    }

    pub fn get_db_handle(&self) -> Arc<Mutex<rusqlite::Connection>> {
        self.storage.connection.clone()
    }
}
