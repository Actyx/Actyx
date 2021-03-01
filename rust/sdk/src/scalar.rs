use crate::types::ArcVal;
use serde::{de::Error, Deserialize, Deserializer};

pub fn nonempty_string<'de, D: Deserializer<'de>>(d: D) -> Result<ArcVal<str>, D::Error> {
    let s = <String>::deserialize(d)?;
    if s.is_empty() {
        Err(D::Error::custom("expected non-empty string"))
    } else {
        Ok(ArcVal::from_boxed(s.into()))
    }
}

macro_rules! mk_scalar {
    ($(#[$attr:meta])* struct $id:ident, $err:ident) => {

        $(#[$attr])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "dataflow", derive(Abomonation))]
        pub struct $id(
            #[serde(deserialize_with = "crate::scalar::nonempty_string")]
            $crate::types::ArcVal<str>
        );

        impl $id {
            pub fn new(value: String) -> Result<Self, $crate::event::ParseError> {
                if value.is_empty() {
                    Err($crate::event::ParseError::$err)
                } else {
                    Ok(Self($crate::types::ArcVal::from_boxed(value.into())))
                }
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
            pub fn as_arc(&self) -> &::std::sync::Arc<str> {
                &self.0.as_arc()
            }
        }

        impl ::std::convert::TryFrom<&str> for $id {
            type Error = $crate::event::ParseError;
            fn try_from(value: &str) -> Result<Self, $crate::event::ParseError> {
                if value.is_empty() {
                    Err($crate::event::ParseError::$err)
                } else {
                    Ok(Self($crate::types::ArcVal::clone_from_unsized(value)))
                }
            }
        }

        impl ::std::convert::TryFrom<::std::sync::Arc<str>> for $id {
            type Error = $crate::event::ParseError;
            fn try_from(value: ::std::sync::Arc<str>) -> Result<Self, $crate::event::ParseError> {
                if value.is_empty() {
                    Err($crate::event::ParseError::$err)
                } else {
                    Ok(Self($crate::types::ArcVal::from(value)))
                }
            }
        }

        impl ::std::str::FromStr for $id {
            type Err = $crate::event::ParseError;
            fn from_str(s: &str) -> Result<Self, $crate::event::ParseError> {
                use std::convert::TryFrom;
                Self::try_from(s)
            }
        }

        impl ::std::ops::Deref for $id {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                self.0.as_ref()
            }
        }

        impl ::std::fmt::Display for $id {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl ::libipld::codec::Decode<::libipld::cbor::DagCborCodec> for $id {
            fn decode<R: ::std::io::Read + ::std::io::Seek>(
                c: ::libipld::cbor::DagCborCodec,
                r: &mut R,
            ) -> ::libipld::error::Result<Self> {
                use ::std::str::FromStr;
                Ok(Self::from_str(&String::decode(c, r)?)?)
            }
        }

        impl ::libipld::codec::Encode<::libipld::cbor::DagCborCodec> for $id {
            fn encode<W: ::std::io::Write>(
                &self,
                c: ::libipld::cbor::DagCborCodec,
                w: &mut W,
            ) -> ::libipld::error::Result<()> {
                use ::std::ops::Deref;
                self.deref().encode(c, w)
            }
        }
    };
}
