use actyxos_sdk::tagged::NodeId;
use anyhow::{Error, Result};
use libp2p::{identity::ed25519, identity::Keypair};
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, fmt};

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

impl From<NodeIdentity> for NodeId {
    fn from(id: NodeIdentity) -> Self {
        crypto::PublicKey::from(id.0.public()).into()
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

impl TryFrom<NodeIdentityIo> for NodeIdentity {
    type Error = Error;
    fn try_from(value: NodeIdentityIo) -> Result<Self, Self::Error> {
        match value {
            NodeIdentityIo::Ed25519 { encoded } => {
                let mut bytes = base64::decode(encoded)?;
                Ok(ed25519::Keypair::decode(&mut bytes).map(NodeIdentity)?)
            }
        }
    }
}

impl std::str::FromStr for NodeIdentity {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let x: NodeIdentityIo = serde_json::from_str(s)?;
        Self::try_from(x)
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
