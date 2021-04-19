use crate::{pair::KeyPair, public::PublicKey};
use anyhow::Result;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug};

/// An Actyx private key.
///
/// Currently this is just a newtype wrapper around an ed25519 private key, but this may
/// change if we ever have the need for another encryption standard.
///
/// It seems like SecretKey is often used in the context of symmetric encryption, so we
/// call this PrivateKey, unlike the wrapped type.
#[derive(Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(from = "ed25519_dalek::SecretKey", into = "ed25519_dalek::SecretKey")]
pub struct PrivateKey(pub(crate) [u8; 32]);

impl Debug for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "secret")
    }
}

impl From<ed25519_dalek::SecretKey> for PrivateKey {
    fn from(value: ed25519_dalek::SecretKey) -> Self {
        PrivateKey(value.to_bytes())
    }
}

impl From<PrivateKey> for ed25519_dalek::SecretKey {
    fn from(value: PrivateKey) -> Self {
        Self::from_bytes(&value.0).expect("wrong secret key value for ed25519_dalek::SecretKey")
    }
}

impl From<PrivateKey> for KeyPair {
    fn from(private: PrivateKey) -> KeyPair {
        let public: PublicKey = private.into();
        KeyPair { public, private }
    }
}
impl From<PrivateKey> for PublicKey {
    fn from(private: PrivateKey) -> PublicKey {
        let secret: ed25519_dalek::SecretKey = private.into();
        let public: ed25519_dalek::PublicKey = (&secret).into();
        public.into()
    }
}

impl PrivateKey {
    pub fn to_ed25519(self) -> ed25519_dalek::SecretKey {
        self.into()
    }
    pub fn to_bytes(&self) -> [u8; ed25519_dalek::SECRET_KEY_LENGTH] {
        self.0
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let ed25519 = ed25519_dalek::SecretKey::from_bytes(bytes)?;
        Ok(ed25519.into())
    }
    pub fn generate() -> Self {
        let k = ed25519_dalek::Keypair::generate(&mut OsRng);
        k.secret.into()
    }
}
