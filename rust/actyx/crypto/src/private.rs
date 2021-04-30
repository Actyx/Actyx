use crate::{pair::KeyPair, public::PublicKey};
use anyhow::{bail, Context, Result};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Debug, Display};

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

impl Display for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let priv_key = self.0;
        let b64 = base64::encode(priv_key);
        write!(f, "0{}", b64)
    }
}

impl std::str::FromStr for PrivateKey {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            bail!("empty string");
        }
        let s = s.as_bytes();
        let key_type = s[0];
        if key_type != b'0' {
            bail!("Unexpected key type {}", key_type);
        }
        let v = base64::decode(&s[1..]).context("error base64 decoding PrivateKey")?;
        if v.len() != ed25519_dalek::SECRET_KEY_LENGTH {
            bail!(
                "Expected {} bytes, received {}",
                ed25519_dalek::SECRET_KEY_LENGTH,
                v.len()
            );
        }
        let mut res = [0u8; ed25519_dalek::SECRET_KEY_LENGTH];
        res.copy_from_slice(&v[..]);
        Ok(Self(res))
    }
}
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

#[cfg(test)]
mod tests {
    use super::PrivateKey;
    use std::str::FromStr;
    #[test]
    fn str_roundtrip() {
        let p = PrivateKey::generate();
        let str = format!("{}", p);
        let round_tripped = PrivateKey::from_str(&str).unwrap();
        assert_eq!(p, round_tripped);
    }
}
