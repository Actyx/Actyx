use crate::internal::arcval::ArcVal;
use crate::parse::ParseError;
use serde::{de::Error, Deserialize, Deserializer, Serialize};
use std::convert::TryFrom;

/// Macro for constructing an [`AppId`](struct.AppId.html) literal.
///
/// This is how it works:
/// ```no_run
/// use ax_sdk::{app_id, AppId};
/// let app_id: AppId = app_id!("abc");
/// ```
/// This does not compile:
/// ```compile_fail
/// use ax_sdk::{app_id, AppId};
/// let app_id: AppId = app_id!("");
/// ```
#[macro_export]
macro_rules! app_id {
    ($lit:tt) => {{
        #[allow(dead_code)]
        type X = ::ax_macros::assert_len!($lit, 1..);
        use ::std::convert::TryFrom;
        $crate::scalars::app_id::AppId::try_from($lit).unwrap()
    }};
}

pub fn is_app_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut dot_allowed = false;
    for c in s.chars() {
        if !c.is_ascii_digit() && !c.is_ascii_lowercase() && c != '-' && (!dot_allowed || c != '.') {
            return false;
        }
        dot_allowed = c != '.';
    }
    dot_allowed
}

pub fn validate_app_id(s: &str) -> Result<(), ParseError> {
    if s.is_empty() {
        return Err(ParseError::EmptyAppId);
    }
    if !is_app_id(s) {
        return Err(ParseError::InvalidAppId(s.to_owned()));
    }
    Ok(())
}

pub fn app_id_string<'de, D: Deserializer<'de>>(d: D) -> Result<ArcVal<str>, D::Error> {
    let s = <String>::deserialize(d)?;
    if s.is_empty() {
        Err(D::Error::custom("expected non-empty string"))
    } else if !is_app_id(&s) {
        Err(D::Error::custom("appId needs to be a valid DNS name"))
    } else {
        Ok(ArcVal::from_boxed(s.into()))
    }
}

/// The app ID denotes a specific app (sans versioning)
///
/// This is used for marking the provenance of events as well as configuring access rights.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AppId(#[serde(deserialize_with = "app_id_string")] ArcVal<str>);

impl AppId {
    pub fn new(value: String) -> Result<Self, ParseError> {
        validate_app_id(&value)?;
        Ok(Self(ArcVal::from_boxed(value.into())))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for AppId {
    type Error = ParseError;
    fn try_from(value: &str) -> Result<Self, ParseError> {
        validate_app_id(value)?;
        Ok(Self(ArcVal::clone_from_unsized(value)))
    }
}

impl TryFrom<std::sync::Arc<str>> for AppId {
    type Error = ParseError;
    fn try_from(value: std::sync::Arc<str>) -> Result<Self, ParseError> {
        validate_app_id(&value)?;
        Ok(Self(ArcVal::from(value)))
    }
}

impl std::str::FromStr for AppId {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, ParseError> {
        Self::try_from(s)
    }
}

impl std::ops::Deref for AppId {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl std::fmt::Display for AppId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

impl libipld::codec::Decode<::libipld::cbor::DagCborCodec> for AppId {
    fn decode<R: std::io::Read + std::io::Seek>(
        c: libipld::cbor::DagCborCodec,
        r: &mut R,
    ) -> libipld::error::Result<Self> {
        use std::str::FromStr;
        Ok(Self::from_str(&String::decode(c, r)?)?)
    }
}

impl libipld::codec::Encode<::libipld::cbor::DagCborCodec> for AppId {
    fn encode<W: std::io::Write>(&self, c: libipld::cbor::DagCborCodec, w: &mut W) -> libipld::error::Result<()> {
        use std::ops::Deref;
        self.deref().encode(c, w)
    }
}

#[cfg(any(test))]
impl quickcheck::Arbitrary for AppId {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        use once_cell::sync::OnceCell;
        static CHOICES: OnceCell<Vec<char>> = OnceCell::new();

        let choices = CHOICES.get_or_init(|| {
            ('a'..='z')
                .chain('0'..='9')
                .chain(std::iter::once('-'))
                .collect::<Vec<_>>()
        });
        let s = Vec::<bool>::arbitrary(g)
            .into_iter()
            .map(|_| *g.choose(choices).unwrap())
            .collect::<String>();
        AppId::try_from(s.as_str()).unwrap_or_else(|_| app_id!("empty"))
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(self.0.shrink().filter(|x| is_app_id(x)).map(Self))
    }
}
