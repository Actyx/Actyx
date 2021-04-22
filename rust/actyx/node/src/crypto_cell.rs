use crate::node_storage::NodeStorage;
use actyxos_sdk::NodeId;
use crypto::KeyStoreRef;
use util::formats::{ActyxOSResult, ActyxOSResultExt};

pub(crate) struct CryptoCell {
    keystore: KeyStoreRef,
    storage: NodeStorage,
}
impl CryptoCell {
    pub fn new(keystore: KeyStoreRef, storage: NodeStorage) -> Self {
        Self { keystore, storage }
    }

    /// Returns this node's NodeId
    pub fn get_or_create_node_id(&self) -> ActyxOSResult<NodeId> {
        if let Some(key_id) = self.storage.get_node_key()? {
            Ok(key_id)
        } else {
            let node_id: NodeId = self.keystore.write().generate_key_pair().ax_internal()?.into();
            self.storage.set_node_id(node_id)?;
            Ok(node_id)
        }
    }
}
