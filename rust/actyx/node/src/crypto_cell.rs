use crate::node_storage::NodeStorage;
use actyxos_sdk::{AppId, NodeId, Timestamp};
use api::{AppMode, BearerToken, Token};
use chrono::{DateTime, Utc};
use crypto::KeyStoreRef;
use util::formats::{ActyxOSResult, ActyxOSResultExt};
use tracing::info;

fn mk_success_log_msg(token: BearerToken) -> String {
    let expiration_time: DateTime<Utc> = token.expiration().into();
    let mode = match token.app_mode {
        AppMode::Trial => "trial",
        // TODO: replace <testing|production> with the right token when we have it
        AppMode::Signed => "<testing|production>",
    };
    format!(
        "Successfully authenticated and authorized {} for {} usage (auth token expires {})",
        token.app_id, mode, expiration_time
    )
}

pub(crate) struct CryptoCell {
    keystore: KeyStoreRef,
    storage: NodeStorage,
}
impl CryptoCell {
    pub fn new(keystore: KeyStoreRef, storage: NodeStorage) -> Self {
        Self { keystore, storage }
    }

    /// Returns a base64 encoded BearerToken. The BearerToken has been signed with this node's key.
    pub fn create_token(
        &self,
        app_id: AppId,
        app_version: String,
        app_mode: AppMode,
        validity: u32,
    ) -> anyhow::Result<Token> {
        let node_key_id = self.get_or_create_node_id()?;
        let cycles = self.storage.get_cycle_count()?;
        let token = BearerToken {
            created: Timestamp::now(),
            app_id,
            cycles,
            app_version,
            validity,
            app_mode,
        };
        let bytes = serde_cbor::to_vec(&token)?;
        let signed = self
            .keystore
            .read()
            .sign(&bytes, std::iter::once(node_key_id.into()))?;
        let log_msg = mk_success_log_msg(token);
        info!(target: "AUTH", "{}", log_msg);
        Ok(base64::encode(signed).into())
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
        let app_id = app_id!("some.app");
        let app_version = "0.1.0".to_string();
        let validity: u32 = 1000;

        let node_storage = NodeStorage::in_memory();
        let keystore = Arc::new(RwLock::new(KeyStore::default()));
        let cell = CryptoCell::new(keystore.clone(), node_storage.clone());
        let token = cell
            .create_token(app_id.clone(), app_version.clone(), AppMode::Signed, validity)
            .unwrap();
        let decoded = base64::decode(token.to_string()).unwrap();
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
        assert!(token.app_id == app_id);
        assert!(token.app_version == app_version);
        assert!(token.validity == validity);
    }
}
