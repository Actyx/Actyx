//! Old data types and conversion methods kept around for backwards
//! compatibility reasons. Don't use anything in here!
use anyhow::{Context, Result};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt::{self, Display},
    io::Read,
};

use crate::{KeyStore, PrivateKey, PublicKey};

fn serialize32<S: Serializer>(t: &[u8; 32], s: S) -> std::result::Result<S::Ok, S::Error> {
    s.serialize_bytes(&t[..])
}
fn deserialize32<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<[u8; 32], D::Error> {
    struct X;
    impl<'de> Visitor<'de> for X {
        type Value = [u8; 32];
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "32 bytes")
        }
        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> std::result::Result<Self::Value, E> {
            if v.len() != 32 {
                return Err(de::Error::custom(format!("found {} bytes", v.len())));
            }
            let mut res = [0u8; 32];
            res.copy_from_slice(v);
            Ok(res)
        }
    }
    d.deserialize_bytes(X)
}
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
/// DONT USE
pub struct LegacyKeyId(#[serde(serialize_with = "serialize32", deserialize_with = "deserialize32")] [u8; 32]);

impl AsRef<[u8]> for LegacyKeyId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl std::str::FromStr for LegacyKeyId {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = base64::decode(s).context("error base64 decoding KeyId")?;
        if v.len() != 32 {
            anyhow::bail!(format!("error parsing KeyId: found {} bytes, expected 32", v.len()));
        }
        let mut res = [0u8; 32];
        res.copy_from_slice(&v[..]);
        Ok(LegacyKeyId(res))
    }
}

impl Display for LegacyKeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", base64::encode(self.0))
    }
}
impl From<&ed25519_dalek::PublicKey> for LegacyKeyId {
    fn from(key: &ed25519_dalek::PublicKey) -> Self {
        Self(Sha256::digest(key.as_bytes()).into())
    }
}
#[derive(Deserialize, Serialize)]
struct KeyPair {
    public: ed25519_dalek::PublicKey,
    private: ed25519_dalek::SecretKey,
}

impl PartialEq for KeyPair {
    fn eq(&self, other: &Self) -> bool {
        self.public == other.public
    }
}
impl Eq for KeyPair {}
#[derive(Deserialize, Serialize, Eq, PartialEq, Debug)]
struct PubKey {
    public: PublicKey,
}
#[derive(Serialize, Deserialize)]
struct LegacyKeyStore {
    pairs: BTreeMap<LegacyKeyId, KeyPair>,
    publics: BTreeMap<LegacyKeyId, PubKey>,
}

impl KeyStore {
    pub(crate) fn restore_legacy_v1(src: impl Read) -> Result<Self> {
        let dec = rust_crypto::aessafe::AesSafe256Decryptor::new(Self::DUMP_KEY);
        let reader = aesstream::AesReader::new(src, dec)?;
        Ok(serde_cbor::from_reader(reader)?)
    }
    pub(crate) fn restore_legacy_v0(src: impl Read) -> Result<Self> {
        let dec = rust_crypto::aessafe::AesSafe256Decryptor::new(Self::DUMP_KEY);
        let reader = aesstream::AesReader::new(src, dec)?;
        let LegacyKeyStore {
            pairs: old_pairs,
            publics: old_publics,
        } = serde_cbor::from_reader(reader)?;
        let pairs = old_pairs
            .into_iter()
            .map(|(_, pair)| {
                let private: PrivateKey = pair.private.into();
                let public: PublicKey = private.into();
                (public, private)
            })
            .collect();
        let publics = old_publics.into_iter().map(|(_, p)| p.public).collect();
        Ok(Self {
            pairs,
            publics,
            dump_after_modify: None,
        })
    }

    pub fn get_public_for_keyid(&self, id: LegacyKeyId) -> Option<PublicKey> {
        self.pairs
            .iter()
            .map(|(p, _)| p)
            .chain(self.publics.iter())
            .find(|p| LegacyKeyId::from(&p.to_ed25519()) == id)
            .copied()
    }
}
