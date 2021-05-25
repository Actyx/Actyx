/*
 * Copyright 2021 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use serde::{de::Error, Deserialize, Deserializer};
use unicode_normalization::UnicodeNormalization;

use crate::types::ArcVal;

pub fn nonempty_string<'de, D: Deserializer<'de>>(d: D) -> Result<ArcVal<str>, D::Error> {
    let s = <String>::deserialize(d)?;
    if s.is_empty() {
        Err(D::Error::custom("expected non-empty string"))
    } else {
        Ok(ArcVal::from_boxed(s.into()))
    }
}

pub fn nonempty_string_canonical<'de, D: Deserializer<'de>>(d: D) -> Result<ArcVal<str>, D::Error> {
    let s = <String>::deserialize(d)?;
    if s.is_empty() {
        Err(D::Error::custom("expected non-empty string"))
    } else {
        Ok(ArcVal::from_boxed(s.nfc().collect::<String>().into()))
    }
}

macro_rules! mk_scalar {
    ($(#[$attr:meta])* struct $id:ident, $err:ident, $parse_err:ident) => {

        $(#[$attr])*
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
        #[cfg_attr(feature = "dataflow", derive(Abomonation))]
        pub struct $id(
            #[serde(deserialize_with = "crate::scalar::nonempty_string")]
            $crate::types::ArcVal<str>
        );

        impl $id {
            pub fn new(value: String) -> Result<Self, $parse_err> {
                if value.is_empty() {
                    Err($parse_err::$err)
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
            type Error = $parse_err;
            fn try_from(value: &str) -> Result<Self, $parse_err> {
                if value.is_empty() {
                    Err($parse_err::$err)
                } else {
                    Ok(Self($crate::types::ArcVal::clone_from_unsized(value)))
                }
            }
        }

        impl ::std::convert::TryFrom<::std::sync::Arc<str>> for $id {
            type Error = $parse_err;
            fn try_from(value: ::std::sync::Arc<str>) -> Result<Self, $parse_err> {
                if value.is_empty() {
                    Err($parse_err::$err)
                } else {
                    Ok(Self($crate::types::ArcVal::from(value)))
                }
            }
        }

        impl ::std::str::FromStr for $id {
            type Err = $parse_err;
            fn from_str(s: &str) -> Result<Self, $parse_err> {
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
