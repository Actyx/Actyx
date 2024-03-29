use anyhow::{anyhow, bail, Context, Result};
use ax_types::NodeId;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    convert::TryFrom,
    fmt::{self, Debug, Display},
    str::FromStr,
};

const DEFAULT_AX_PUBLIC_KEY_B64: &str = "075i62XGQJuXjv6nnLQyJzECZhF29acYvYeEOJ3kc5M8=";

/// A public key, which also serves as identifier for the corresponding private key
///
/// It consists of 32 octets which are actually the same bytes as the underlying `ed25519_dalek::PublicKey`. Thus
/// it's possible to derive all sorts of other identifier from this structure, like a `libp2p::PeerId`.
///
/// A general representation is achieved by base64-encoding the bytes, and prepending an identifier for
/// the key type, which at the moment is only a literal '0' to identify it as an `ed25519_dalek::PublicKey`.
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct PublicKey(pub(crate) [u8; 32]);

impl Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pub_key = self.0;
        let b64 = base64::encode(pub_key);
        write!(f, "0{}", b64)
    }
}

impl Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl FromStr for PublicKey {
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
        let v = base64::decode(&s[1..]).context("error base64 decoding PubKey")?;
        if v.len() != ed25519_dalek::PUBLIC_KEY_LENGTH {
            bail!(
                "Expected {} bytes, received {}",
                ed25519_dalek::PUBLIC_KEY_LENGTH,
                v.len()
            );
        }

        let mut res = [0u8; ed25519_dalek::PUBLIC_KEY_LENGTH];
        res.copy_from_slice(&v[..]);

        // validate if bytes are decompressable into EdwardPoints
        ed25519_dalek::PublicKey::from_bytes(&res).map_err(|err| anyhow!(err))?;

        Ok(Self(res))
    }
}

impl PublicKey {
    pub(crate) fn ax_public_key() -> Self {
        Self::from_str(option_env!("AX_PUBLIC_KEY").unwrap_or(DEFAULT_AX_PUBLIC_KEY_B64))
            .expect("The default key should be a valid public key")
    }
    /// Gets the underlying ed25519 public key for interop with rust crypto libs
    pub fn to_ed25519(self) -> ed25519_dalek::PublicKey {
        ed25519_dalek::PublicKey::from_bytes(&self.0[..]).unwrap()
    }
    pub fn to_bytes(self) -> [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] {
        let mut bytes = [0u8; ed25519_dalek::PUBLIC_KEY_LENGTH];
        bytes[..].copy_from_slice(&self.0[..]);
        bytes
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let ed25519 = ed25519_dalek::PublicKey::from_bytes(bytes)?;
        Ok(ed25519.into())
    }
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        let signature = if let Ok(sig) = ed25519_dalek::Signature::try_from(signature) {
            sig
        } else {
            return false;
        };
        use ed25519_dalek::Verifier;
        self.to_ed25519().verify(message, &signature).is_ok()
    }
}

pub fn node_id_to_peer_id(node_id: NodeId) -> libp2p::core::PeerId {
    let public: PublicKey = node_id.into();
    public.into()
}

pub fn peer_id_to_node_id(peer_id: libp2p::core::PeerId) -> anyhow::Result<NodeId> {
    let public = PublicKey::try_from(peer_id)?;
    Ok(public.into())
}

impl From<PublicKey> for libp2p::core::PeerId {
    fn from(pb: PublicKey) -> libp2p::core::PeerId {
        let public = pb.into();
        libp2p::core::PeerId::from_public_key(&public)
    }
}

impl TryFrom<libp2p::core::PeerId> for PublicKey {
    type Error = anyhow::Error;
    // This only works if the multi_hash with this peer_id is encoded is the identity hash,
    // and the underlying public key is ed25519
    fn try_from(peer_id: libp2p::core::PeerId) -> Result<Self, Self::Error> {
        match multihash::Multihash::from_bytes(&peer_id.to_bytes()) {
            Ok(multihash) => {
                if multihash.code() == u64::from(multihash::Code::Identity) {
                    let bytes = multihash.digest();
                    let libp2p_pubkey = libp2p::core::identity::PublicKey::from_protobuf_encoding(bytes)?;
                    match libp2p_pubkey {
                        libp2p::core::identity::PublicKey::Ed25519(ed25519_pub) => {
                            let bytes = ed25519_pub.encode();

                            let pub_key = ed25519_dalek::PublicKey::from_bytes(&bytes[..])
                                .map_err(|e| anyhow!(e))
                                .context("Not a valid ed25519_dalek::PublicKey")?;
                            Ok(pub_key.into())
                        } // _ => bail!("Expected ed25519_dalek::PublicKey!"),
                    }
                } else {
                    bail!("Only PeerIds encoded with identity hash can be decoded")
                }
            }

            Err(err) => bail!(err),
        }
    }
}

impl TryFrom<&libp2p::identity::PublicKey> for PublicKey {
    type Error = anyhow::Error;

    fn try_from(value: &libp2p::identity::PublicKey) -> Result<Self, Self::Error> {
        match value {
            libp2p::core::PublicKey::Ed25519(key) => Ok(key.into()),
        }
    }
}

impl From<PublicKey> for libp2p::core::identity::PublicKey {
    fn from(pk: PublicKey) -> libp2p::core::identity::PublicKey {
        libp2p::core::identity::PublicKey::Ed25519(
            libp2p::core::identity::ed25519::PublicKey::decode(&pk.0)
                .expect("ed25519 encoding format changed between libp2p and crypto"),
        )
    }
}

impl From<libp2p::core::identity::ed25519::PublicKey> for PublicKey {
    fn from(o: libp2p::core::identity::ed25519::PublicKey) -> Self {
        Self(o.encode())
    }
}

impl From<&libp2p::core::identity::ed25519::PublicKey> for PublicKey {
    fn from(o: &libp2p::core::identity::ed25519::PublicKey) -> Self {
        Self(o.encode())
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<ed25519_dalek::PublicKey> for PublicKey {
    fn from(key: ed25519_dalek::PublicKey) -> Self {
        Self(*key.as_bytes())
    }
}

impl From<NodeId> for PublicKey {
    fn from(node_id: NodeId) -> Self {
        let mut res = [0u8; ed25519_dalek::PUBLIC_KEY_LENGTH];
        res.copy_from_slice(node_id.as_ref());
        Self(res)
    }
}

impl From<PublicKey> for NodeId {
    fn from(p: PublicKey) -> NodeId {
        NodeId::from_bytes(p.as_ref()).unwrap()
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = PublicKey;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("PublicKey")
            }
            fn visit_str<E: serde::de::Error>(self, string: &str) -> Result<Self::Value, E> {
                PublicKey::from_str(string).map_err(serde::de::Error::custom)
            }
        }
        deserializer.deserialize_str(V)
    }
}

#[cfg(test)]
mod tests {
    use super::PublicKey;
    use crate::crypto::PrivateKey;
    use std::str::FromStr;
    #[test]
    fn str_roundtrip() {
        let private = PrivateKey::generate();
        let p: PublicKey = private.into();
        let str = format!("{}", p);
        let round_tripped = PublicKey::from_str(&str).unwrap();
        assert_eq!(p, round_tripped);
    }
}
