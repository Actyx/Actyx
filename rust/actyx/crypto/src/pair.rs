use crate::{private::PrivateKey, public::PublicKey};
use actyx_sdk::NodeId;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};

/// A keypair.
///
/// Conceptually, this is a generic keypair. But currently we only support ed25519 encryption.
#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct KeyPair {
    pub(crate) public: PublicKey,
    pub(crate) private: PrivateKey,
}

impl PartialEq for KeyPair {
    fn eq(&self, other: &Self) -> bool {
        self.public == other.public
    }
}
impl Eq for KeyPair {}

impl Debug for KeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyPair").field("public", &self.public).finish()
    }
}

impl From<KeyPair> for libp2p::core::identity::Keypair {
    fn from(kp: KeyPair) -> libp2p::core::identity::Keypair {
        libp2p::core::identity::Keypair::Ed25519(kp.into())
    }
}

impl From<KeyPair> for ed25519_dalek::Keypair {
    fn from(kp: KeyPair) -> Self {
        Self {
            public: kp.public.to_ed25519(),
            secret: kp.private.into(),
        }
    }
}

impl From<KeyPair> for NodeId {
    fn from(kp: KeyPair) -> Self {
        kp.public.into()
    }
}

impl From<KeyPair> for libp2p::PeerId {
    fn from(kp: KeyPair) -> Self {
        libp2p::core::identity::Keypair::from(kp).public().to_peer_id()
    }
}

impl KeyPair {
    pub fn generate() -> Self {
        PrivateKey::generate().into()
    }

    pub fn pub_key(&self) -> PublicKey {
        self.public
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let secret_key = self.private.to_ed25519();
        ed25519_dalek::ExpandedSecretKey::from(&secret_key)
            .sign(message, &self.public.to_ed25519())
            .to_bytes()
    }

    /// Convert this keypair to bytes.
    ///
    /// # Returns
    //
    /// An array of bytes, `[u8; KEYPAIR_LENGTH]`.  The first
    /// `SECRET_KEY_LENGTH` of bytes is the `SecretKey`, and the next
    /// `PUBLIC_KEY_LENGTH` bytes is the `PublicKey` (the same as other
    /// libraries, such as [Adam Langley's ed25519 Golang
    /// implementation](https://github.com/agl/ed25519/)).
    ///
    /// Copied from ed2559_dalek::KeyPair::to_bytes
    pub fn to_bytes(self) -> [u8; ed25519_dalek::PUBLIC_KEY_LENGTH + ed25519_dalek::SECRET_KEY_LENGTH] {
        let mut bytes: [u8; 64] = [0u8; 64];

        bytes[..ed25519_dalek::SECRET_KEY_LENGTH].copy_from_slice(&self.private.0);
        bytes[ed25519_dalek::SECRET_KEY_LENGTH..].copy_from_slice(&self.public.0);
        bytes
    }
}
