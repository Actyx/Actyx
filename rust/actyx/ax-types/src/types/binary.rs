use base64::DecodeError;
use derive_more::{From, Into};
use fmt::Display;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{
    fmt::{self, Formatter},
    str::FromStr,
};

#[derive(Debug, Clone, From, Into, Ord, PartialOrd, Eq, PartialEq)]
pub struct Binary(Box<[u8]>);

struct X;
impl<'de> Visitor<'de> for X {
    type Value = Binary;
    fn expecting(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str("binary data (possibly base64-encoded)")
    }
    fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
        Ok(Binary(v.into()))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        let bytes = base64::decode(v).map_err(|e| E::custom(e.to_string()))?;
        Ok(Binary(bytes.into()))
    }
}

impl<'de> Deserialize<'de> for Binary {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(X)
    }
}

impl Serialize for Binary {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(base64::encode(&*self.0).as_str())
        } else {
            serializer.serialize_bytes(&self.0)
        }
    }
}

impl FromStr for Binary {
    type Err = DecodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = base64::decode(s)?;
        Ok(Binary(bytes.into()))
    }
}

impl AsRef<[u8]> for Binary {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Display for Binary {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(base64::encode(&*self.0).as_str())
    }
}

impl From<&[u8]> for Binary {
    fn from(v: &[u8]) -> Self {
        Binary(v.into())
    }
}

impl From<Vec<u8>> for Binary {
    fn from(v: Vec<u8>) -> Self {
        Binary(v.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn must_serialize_json() {
        let binary = Binary((&[1, 2, 3, 255][..]).into());
        let text = serde_json::to_string(&binary).unwrap();
        assert_eq!(text, r#""AQID/w==""#);
        assert_eq!(serde_json::from_str::<Binary>(&text).unwrap(), binary);
    }

    #[test]
    fn must_serialize_cbor() {
        let binary = Binary((&[1, 2, 3][..]).into());
        let cbor = serde_cbor::to_vec(&binary).unwrap();
        assert_eq!(cbor, vec![67, 1, 2, 3]);
        assert_eq!(serde_cbor::from_slice::<Binary>(&cbor).unwrap(), binary);
    }
}
