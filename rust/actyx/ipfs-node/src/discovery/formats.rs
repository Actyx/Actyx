use derive_more::{From, Into};
use libp2p::{Multiaddr, PeerId};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::{convert::TryFrom, str::FromStr};

#[derive(PartialEq, Eq, Hash, From, Into)]
pub struct MultiaddrIo(pub Multiaddr);

impl std::cmp::Ord for MultiaddrIo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let lhs = self.0.as_ref();
        let rhs = other.0.as_ref();
        lhs.cmp(&rhs)
    }
}

impl std::cmp::PartialOrd for MultiaddrIo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for MultiaddrIo {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.0.to_string())
        } else {
            serializer.serialize_bytes(&self.0.to_vec())
        }
    }
}

impl<'de> Deserialize<'de> for MultiaddrIo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MyVisitor;

        impl<'de> Visitor<'de> for MyVisitor {
            type Value = MultiaddrIo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Multiaddr")
            }

            fn visit_bytes<E: serde::de::Error>(self, value: &[u8]) -> Result<MultiaddrIo, E> {
                Multiaddr::try_from(value.to_owned())
                    .map(MultiaddrIo)
                    .map_err(serde::de::Error::custom)
            }

            fn visit_str<E: serde::de::Error>(self, string: &str) -> Result<Self::Value, E> {
                Multiaddr::from_str(string)
                    .map(MultiaddrIo)
                    .map_err(serde::de::Error::custom)
            }
        }
        deserializer.deserialize_any(MyVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, From, Into)]
pub struct PeerIdIo(pub PeerId);

impl Serialize for PeerIdIo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.0.to_string())
        } else {
            serializer.serialize_bytes(&self.0.to_bytes())
        }
    }
}

impl<'de> Deserialize<'de> for PeerIdIo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MyVisitor;

        impl<'de> Visitor<'de> for MyVisitor {
            type Value = PeerIdIo;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("PeerId")
            }

            fn visit_str<E: serde::de::Error>(self, string: &str) -> Result<Self::Value, E> {
                PeerId::from_str(string).map(PeerIdIo).map_err(serde::de::Error::custom)
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<PeerIdIo, E>
            where
                E: serde::de::Error,
            {
                PeerId::try_from(value.to_owned())
                    .map(PeerIdIo)
                    .map_err(|_| serde::de::Error::custom("invalid peer id"))
            }
        }
        deserializer.deserialize_any(MyVisitor)
    }
}
