use crate::discovery::DiscoveryConfig;
use actyxos_sdk::tagged::NodeId;
use anyhow::Result;
use derive_more::{Display, Error, From};
use libp2p::{
    gossipsub::{GossipsubConfig, GossipsubConfigBuilder, ValidationMode},
    identity,
    identity::ed25519,
    identity::Keypair,
    ping::PingConfig,
    pnet::PreSharedKey,
    Multiaddr,
};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fmt, num::NonZeroU32, path::PathBuf, str::FromStr, time::Duration};

#[derive(Debug, Clone)]
pub struct NodeIdentity(ed25519::Keypair);

impl NodeIdentity {
    pub fn to_keypair(&self) -> Keypair {
        Keypair::Ed25519(self.0.clone())
    }

    pub fn generate() -> Self {
        Self(ed25519::Keypair::generate())
    }
}

impl Into<NodeId> for NodeIdentity {
    fn into(self) -> NodeId {
        let p = crypto::PublicKey::from(self.0.public());
        p.into()
    }
}

impl PartialEq for NodeIdentity {
    fn eq(&self, that: &NodeIdentity) -> bool {
        self.0.encode().as_ref() == that.0.encode().as_ref()
    }
}

impl Eq for NodeIdentity {}

impl fmt::Display for NodeIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&NodeIdentityIo::from(self.clone())).map_err(|_| std::fmt::Error)?
        )
    }
}

/// This enum with one case is so the serialized format is future proof in case we ever want something else than Ed25519
#[derive(Debug, Clone, Serialize, Deserialize)]
enum NodeIdentityIo {
    Ed25519 {
        /// base64 encoded 64 byte blob that contains the ed25519 parameters
        encoded: String,
    },
}

impl From<NodeIdentity> for NodeIdentityIo {
    fn from(value: NodeIdentity) -> Self {
        NodeIdentityIo::Ed25519 {
            encoded: base64::encode(value.0.encode().as_ref()),
        }
    }
}

impl From<crypto::KeyPair> for NodeIdentity {
    fn from(crypto_kp: crypto::KeyPair) -> Self {
        Self(crypto_kp.into())
    }
}

#[derive(Debug, Display, From, Error)]
pub enum NodeIdentityDecodeError {
    Base64DecodeError(base64::DecodeError),
    KeypairDecodeError(identity::error::DecodingError),
}

impl TryFrom<NodeIdentityIo> for NodeIdentity {
    type Error = NodeIdentityDecodeError;
    fn try_from(value: NodeIdentityIo) -> Result<Self, Self::Error> {
        match value {
            NodeIdentityIo::Ed25519 { encoded } => {
                let mut bytes = base64::decode(encoded)?;
                Ok(ed25519::Keypair::decode(&mut bytes).map(NodeIdentity)?)
            }
        }
    }
}

#[derive(Debug, Display, From, Error)]
pub enum NodeIdentityParseError {
    JsonError(serde_json::Error),
    NodeIdentityDecodeError(NodeIdentityDecodeError),
}

impl std::str::FromStr for NodeIdentity {
    type Err = NodeIdentityParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let x: NodeIdentityIo = serde_json::from_str(s)?;
        Ok(Self::try_from(x)?)
    }
}

impl FromSql for NodeIdentity {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> FromSqlResult<Self> {
        value
            .as_str()
            .and_then(|data| NodeIdentity::from_str(data).map_err(|e| FromSqlError::Other(Box::new(e))))
    }
}

impl ToSql for NodeIdentity {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

/// configuration for an actyx ipfs node
#[derive(Clone)]
pub struct NodeConfig {
    /// the key pair. The peer id is derived from this
    pub local_key: NodeIdentity,

    /// an optional pre shared key for private swarms
    pub pre_shared_key: Option<PreSharedKey>,

    /// gossipsub configuration
    ///
    /// Note that we only support gossipsub for now. There is no fallback to floodsub. So
    /// remote nodes *have* to be configured for pubsub to work.
    ///
    /// In the go-ipfs config, this would be
    /// ```json
    ///  "Pubsub": {
    ///    "Router": "gossipsub",
    ///  }
    /// ```
    pub gossipsub_config: GossipsubConfig,

    /// list of bootstrap addresses. These have to be just endpoint addresses without peer id._
    ///
    /// E.g. to dial to a node running on localhost, use `/ip4/127.0.0.1/tcp/4001`
    pub bootstrap: Vec<Multiaddr>,

    /// manually configured list of own addresses to announce
    ///
    /// addresses under which the node is reachable, that it can not figure out itself.
    /// if this is non-empty, the automatic mechanism to determine own addresses will be disabled,
    /// and these will be the only addresses to be announced.
    pub announce: Vec<Multiaddr>,

    /// list of listen addresses
    ///
    /// e.g. `/ip4/0.0.0.0/tcp/0` to listen on a random port on all interfaces
    pub listen: Vec<Multiaddr>,

    /// block store db path
    pub block_store_path: Option<PathBuf>,

    /// block store cache size.
    pub block_store_size: u64,

    /// config for the discovery mechanism
    pub discovery_config: DiscoveryConfig,

    /// true to switch on mdns
    pub use_mdns: bool,

    /// allow publish
    pub allow_publish: bool,

    /// use dev transport for testing
    pub enable_dev_transport: bool,

    /// Timeout for the upgrade of a connection from a raw connection to an encrypted
    /// and multiplexed connection. 20s should be enough for a few roundtrips unless
    /// the underlying transport is really high latency.
    pub upgrade_timeout: Duration,

    /// Ping config.
    pub ping_config: PingConfig,
}

impl fmt::Debug for NodeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeConfig")
            .field("peer_id", &self.local_key.to_keypair().public().into_peer_id())
            // do not print the psk itself but just the fingerprint, so we don't leak the
            // psk all over the log files!
            .field(
                "psk_fingerprint",
                &self.pre_shared_key.map(|psk| psk.fingerprint().to_string()),
            )
            .field("bootstrap", &self.bootstrap)
            .field("listen", &self.listen)
            .field("announce", &self.announce)
            .field("block_store_path", &self.block_store_path)
            .field("block_store_size", &self.block_store_size)
            .field("discovery_config", &self.discovery_config)
            .field("use_mdns", &self.use_mdns)
            .field("allow_publish", &self.allow_publish)
            .field("enable_dev_transport", &self.enable_dev_transport)
            .field("upgrade_timeout", &self.upgrade_timeout)
            .field("ping_config", &self.ping_config)
            .finish()
    }
}

impl NodeConfig {
    pub fn new(config: ax_config::IpfsNodeConfig) -> Result<Self> {
        let pre_shared_key = if let Some(psk) = config.pre_shared_key {
            let blob = base64::decode(psk)?;
            let decoded = String::from_utf8(blob)?;
            Some(PreSharedKey::from_str(&decoded)?)
        } else {
            None
        };
        let local_key = if let Some(identity) = config.identity {
            NodeIdentity::from_str(&identity)?
        } else {
            NodeIdentity::generate()
        };
        let block_store_size = config.db_size.unwrap_or(1024 * 1024 * 1024 * 4);
        let gossipsub_config = GossipsubConfigBuilder::default()
            // Increase the max msg size because the default is very small.
            .max_transmit_size(262_144)
            .validation_mode(ValidationMode::Permissive)
            .build()
            .expect("valid gossipsub config");
        let ping_config = PingConfig::new()
            .with_keep_alive(true)
            .with_max_failures(NonZeroU32::new(2).unwrap());
        Ok(Self {
            gossipsub_config,
            local_key,
            pre_shared_key,
            bootstrap: config.bootstrap,
            listen: config.listen,
            announce: config.external_addresses,
            block_store_path: config.db_path,
            block_store_size,
            discovery_config: DiscoveryConfig::default(),
            use_mdns: config.enable_mdns,
            allow_publish: config.enable_publish,
            enable_dev_transport: false,
            upgrade_timeout: Duration::from_secs(20),
            ping_config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::NodeIdentity;
    use crypto::KeyStore;
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;
    use std::str::FromStr;

    #[test]
    fn deser_ser_roundtrip() -> anyhow::Result<()> {
        let reference = r#"{"Ed25519":{"encoded":"VswJ+UwXZpVCRcTWlr2pTJRvfBsXKKb8u+kRxz4FArif2LsSilZspNRN2JaaDbu/5bUxLkoKSz7Lv0ZlbpXL0Q=="}}"#;
        let deser = NodeIdentity::from_str(reference)?;
        let peer = deser.to_keypair().public().into_peer_id();
        let actual = deser.to_string();
        assert_eq!(
            "12D3KooWLaLfqv5L7rL4NE6WYTGVFWLD8HKaDNTdBA97cMLfzA4x",
            &peer.to_string()
        );
        assert_eq!(reference, &actual);
        Ok(())
    }

    impl Arbitrary for NodeIdentity {
        fn arbitrary(_: &mut Gen) -> Self {
            Self::generate()
        }
    }

    #[quickcheck]
    fn node_idenity_serde_roundtrip(reference: NodeIdentity) -> anyhow::Result<bool> {
        let text = reference.to_string();
        let actual = NodeIdentity::from_str(&text)?;
        Ok(reference == actual)
    }

    #[test]
    fn node_identity_keystore_roundtrip() -> anyhow::Result<()> {
        let mut ks = KeyStore::default();
        let key_id = ks.generate_key_pair()?;
        let kp = ks.get_pair(key_id).unwrap();
        let identity: NodeIdentity = kp.into();
        let libp2p_kp = identity.to_keypair();

        let my_message = [0u8, 1u8, 2u8];
        let signed = ks.sign(&my_message, std::iter::once(key_id))?;
        assert!(libp2p_kp.public().verify(signed.message(), signed.signatures()[0].1));
        Ok(())
    }
}
