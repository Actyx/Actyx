use serde::de::{self, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Base64Blob(pub Vec<u8>);

impl Serialize for Base64Blob {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(base64::encode(&self.0).as_ref())
    }
}

impl<'de> Deserialize<'de> for Base64Blob {
    fn deserialize<D>(deserializer: D) -> Result<Base64Blob, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MyVisitor();

        impl<'de> Visitor<'de> for MyVisitor {
            type Value = Base64Blob;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                base64::decode(v)
                    .map(Base64Blob)
                    .map_err(|err| serde::de::Error::custom(format!("Error decoding base64 string: {}", err)))
            }
        }

        deserializer.deserialize_any(MyVisitor())
    }
}
