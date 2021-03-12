use crate::node_storage::NodeStorage;
use actyxos_lib::{ActyxOSResult, ActyxOSResultExt, AppId};
use actyxos_sdk::{
    tagged::{self, NodeId},
    TimeStamp,
};
use crypto::KeyStoreRef;
use trees::BearerToken;

pub(crate) struct CryptoCell {
    keystore: KeyStoreRef,
    storage: NodeStorage,
}
impl CryptoCell {
    pub fn new(keystore: KeyStoreRef, storage: NodeStorage) -> Self {
        Self { keystore, storage }
    }
    /// Returns a base64 encoded BearerToken. The BearerToken has been signed with this node's key.
    #[allow(dead_code)]
    pub fn create_token(&self, app_id: AppId) -> ActyxOSResult<String> {
        let node_key_id = self.get_or_create_node_id()?;
        let cycles = self.storage.get_cycle_count()?;

        let token = BearerToken {
            created: TimeStamp::now(),
            app_id: tagged::AppId::new(app_id.into()).ax_internal()?,
            cycles,
            version: env!("CARGO_PKG_VERSION").into(),
            validity: u32::MAX,
        };
        let mut msg = vec![];
        serde_cbor::to_writer(&mut msg, &token).ax_internal()?;
        let signed = self
            .keystore
            .read()
            .sign(&msg, std::iter::once(node_key_id.into()))
            .ax_internal()?;
        Ok(base64::encode(signed))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_storage::NodeStorage;
    use actyxos_sdk::app_id;
    use crypto::{KeyStore, SignedMessage};
    use parking_lot::RwLock;
    use std::{convert::TryFrom, sync::Arc};

    #[test]
    fn should_create_tokens() {
        let node_storage = NodeStorage::in_memory();
        let keystore = Arc::new(RwLock::new(KeyStore::default()));
        let cell = CryptoCell::new(keystore.clone(), node_storage.clone());
        let token = cell.create_token("some.app".into()).unwrap();
        let decoded = base64::decode(token).unwrap();
        let signed_message = SignedMessage::try_from(&decoded[..]).unwrap();
        keystore
            .read()
            .verify(
                &signed_message,
                std::iter::once(node_storage.get_node_key().unwrap().unwrap().into()),
            )
            .unwrap();
        let token: BearerToken = serde_cbor::from_slice(signed_message.message()).unwrap();
        assert!(token.cycles == 0);
        assert!(token.app_id == app_id!("some.app"));
        assert!(token.version == env!("CARGO_PKG_VERSION"));
        assert!(token.validity == u32::MAX);
    }
}
