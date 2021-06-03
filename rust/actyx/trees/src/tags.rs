use std::{convert::TryInto, io, iter::FromIterator};

use actyxos_sdk::{Tag, TagSet};
use libipld::{
    cbor::DagCborCodec,
    codec::{Decode, Encode},
};
use serde::{Deserialize, Serialize};
use vec_collections::VecSet2;

#[macro_export]
macro_rules! stags {
    ($($args:tt)*) => {{
        let res = ::actyxos_sdk::tags!($($args)*);
        crate::tags::ScopedTagSet::from(res)
    }};
}

#[derive(Debug, PartialOrd, Ord, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagScope {
    /// Internal scope, '!' prefix
    Internal,
    /// App scope,'_' prefix
    App,
}

#[derive(Debug, PartialOrd, Ord, Clone, PartialEq, Eq, Hash)]
pub struct ScopedTag(TagScope, Tag);

impl ScopedTag {
    pub fn app(&self) -> Option<&Tag> {
        match self.0 {
            TagScope::App => Some(&self.1),
            _ => None,
        }
    }

    pub fn into_app(self) -> Option<Tag> {
        match self.0 {
            TagScope::App => Some(self.1),
            _ => None,
        }
    }

    fn to_serialized(&self) -> String {
        format!("{}{}", self.0.prefix(), self.1)
    }

    fn parse(text: &str) -> anyhow::Result<Self> {
        anyhow::ensure!(text.len() > 1);
        Ok(match &text.as_bytes()[..1] {
            b"_" => ScopedTag(TagScope::App, text[1..].try_into().unwrap()),
            b"!" => ScopedTag(TagScope::Internal, text[1..].try_into().unwrap()),
            prefix => anyhow::bail!("unknown prefix {:?} for tag {}", prefix, text),
        })
    }
}

impl Encode<DagCborCodec> for ScopedTag {
    fn encode<W: io::Write>(&self, c: DagCborCodec, w: &mut W) -> anyhow::Result<()> {
        self.to_serialized().encode(c, w)
    }
}

impl Decode<DagCborCodec> for ScopedTag {
    fn decode<R: io::Read + io::Seek>(c: DagCborCodec, r: &mut R) -> anyhow::Result<Self> {
        Self::parse(&String::decode(c, r)?)
    }
}

impl Serialize for ScopedTag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_serialized().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ScopedTag {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl From<Tag> for ScopedTag {
    fn from(tag: Tag) -> Self {
        Self(TagScope::App, tag)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedTagSet(VecSet2<ScopedTag>);

impl ScopedTagSet {
    pub fn empty() -> Self {
        Self(VecSet2::empty())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn is_subset(&self, that: &Self) -> bool {
        self.0.is_subset(&that.0)
    }

    pub fn public_tags(&self) -> impl Iterator<Item = &Tag> {
        self.0.iter().filter_map(|t| t.app())
    }
}

impl std::ops::BitOrAssign for ScopedTagSet {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0
    }
}

impl From<Vec<ScopedTag>> for ScopedTagSet {
    fn from(tags: Vec<ScopedTag>) -> Self {
        Self(tags.into_iter().collect())
    }
}

impl AsRef<[ScopedTag]> for ScopedTagSet {
    fn as_ref(&self) -> &[ScopedTag] {
        self.0.as_ref()
    }
}

impl FromIterator<ScopedTag> for ScopedTagSet {
    fn from_iter<T: IntoIterator<Item = ScopedTag>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl From<TagSet> for ScopedTagSet {
    fn from(value: TagSet) -> Self {
        Self(value.into_iter().map(Into::into).collect())
    }
}

impl IntoIterator for ScopedTagSet {
    type Item = ScopedTag;

    type IntoIter = vec_collections::VecSetIter<smallvec::IntoIter<[ScopedTag; 2]>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl TagScope {
    /// Get the prefix for a scope.
    pub const fn prefix(&self) -> &str {
        match self {
            Self::App => "_",
            Self::Internal => "!",
        }
    }
}
