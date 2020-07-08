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
            pub fn new(value: String) -> Result<Self, ParseError> {
                if value.is_empty() {
                    Err(ParseError::$err)
                } else {
                    Ok(Self(ArcVal::from_boxed(value.into())))
                }
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
            pub fn as_arc(&self) -> &Arc<str> {
                &self.0.as_arc()
            }
        }

        impl TryFrom<&str> for $id {
            type Error = ParseError;
            fn try_from(value: &str) -> Result<Self, ParseError> {
                if value.is_empty() {
                    Err(ParseError::$err)
                } else {
                    Ok(Self(ArcVal::clone_from_unsized(value)))
                }
            }
        }

        impl TryFrom<Arc<str>> for $id {
            type Error = ParseError;
            fn try_from(value: Arc<str>) -> Result<Self, ParseError> {
                if value.is_empty() {
                    Err(ParseError::$err)
                } else {
                    Ok(Self(ArcVal::from(value)))
                }
            }
        }

        impl Deref for $id {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                self.0.as_ref()
            }
        }
    };
}
