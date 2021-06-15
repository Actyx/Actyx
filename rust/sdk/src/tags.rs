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
use std::{
    collections::BTreeSet,
    convert::TryFrom,
    fmt, io,
    iter::FromIterator,
    ops::{Add, AddAssign, BitAndAssign, SubAssign},
    str::FromStr,
    sync::Arc,
};

use libipld::{
    cbor::DagCborCodec,
    codec::{Decode, Encode},
    DagCbor,
};
use serde::{Deserialize, Serialize};

use crate::{types::ArcVal, ParseError};
use unicode_normalization::UnicodeNormalization;

/// Macro for constructing a [`Tag`](struct.Tag.html) literal.
///
/// This is how it works:
/// ```no_run
/// use actyx_sdk::{tag, Tag};
/// let tag: Tag = tag!("abc");
/// ```
/// This does not compile:
/// ```compile_fail
/// use actyx_sdk::{tag, tags::Tag};
/// let tag: Tag = tag!("");
/// ```
#[macro_export]
macro_rules! tag {
    ($lit:tt) => {{
        #[allow(dead_code)]
        type X = $crate::assert_len!($lit, 1..);
        use std::convert::TryFrom;
        $crate::Tag::try_from($lit).unwrap()
    }};
}

/// Macro for constructing a set of [`Tag`](struct.Tag.html) values.
///
/// The values accepted are either
///  - non-empty string literals
///  - normal expressions (enclosed in parens if multiple tokens)
///
/// ```rust
/// use actyx_sdk::{semantics, tag, Tag, tags, TagSet};
/// use std::collections::BTreeSet;
///
/// let tags: TagSet = tags!("a", "semantics:b");
/// let mut expected = BTreeSet::new();
/// expected.insert(tag!("a"));
/// expected.insert(tag!("semantics:b"));
/// assert_eq!(tags, TagSet::from(expected));
/// ```
#[macro_export]
macro_rules! tags {
    () => { $crate::TagSet::empty() };
    ($($expr:expr),*) => {{
        let mut tags = Vec::new();
        $(
            {
                mod y {
                    $crate::assert_len! { $expr, 1..,
                        // if it is a string literal, then we know it is not empty
                        pub fn x(z: &str) -> $crate::Tag {
                            use ::std::convert::TryFrom;
                            $crate::Tag::try_from(z).unwrap()
                        },
                        // if it is not a string literal, require an infallible conversion
                        pub fn x(z: impl Into<$crate::Tag>) -> $crate::Tag {
                            z.into()
                        }
                    }
                }
                tags.push(y::x($expr));
            }
        )*
        $crate::TagSet::from(tags)
    }};
    ($($x:tt)*) => {
        compile_error!("This macro supports only string literals or expressions in parens.")
    }
}

/// A Tag that semantically characterises an event.
///
/// Tags are non-empty unicode strings in NFC representation (i.e. normalized by canonical decomposition
/// followed by composition). Thus, `ℌ` and `H` are different tags while the various encodings of `é` are
/// all represented by the codepoint E9.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct Tag(#[serde(deserialize_with = "crate::scalar::nonempty_string_canonical")] ArcVal<str>);

#[allow(clippy::len_without_is_empty)]
impl Tag {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl TryFrom<&str> for Tag {
    type Error = ParseError;
    fn try_from(value: &str) -> std::result::Result<Self, ParseError> {
        Self::from_str(value)
    }
}

impl TryFrom<Arc<str>> for Tag {
    type Error = ParseError;
    fn try_from(value: Arc<str>) -> std::result::Result<Self, ParseError> {
        Self::from_str(value.as_ref())
    }
}

impl FromStr for Tag {
    type Err = ParseError;
    fn from_str(s: &str) -> std::result::Result<Self, ParseError> {
        if s.is_empty() {
            Err(ParseError::EmptyTag)
        } else {
            Ok(Self(crate::types::ArcVal::from_boxed(
                s.nfc().collect::<String>().into_boxed_str(),
            )))
        }
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Decode<DagCborCodec> for Tag {
    fn decode<R: io::Read + io::Seek>(c: DagCborCodec, r: &mut R) -> ::libipld::error::Result<Self> {
        Ok(Self::from_str(&String::decode(c, r)?)?)
    }
}

impl Encode<DagCborCodec> for Tag {
    fn encode<W: io::Write>(&self, c: DagCborCodec, w: &mut W) -> ::libipld::error::Result<()> {
        self.0.encode(c, w)
    }
}

/// Concatenate another part to this tag
///
/// ```
/// # use actyx_sdk::{tag, Tag};
/// let user_tag = tag!("user:") + "Bob";
/// let machine_tag = tag!("machine:") + format!("{}-{}", "thing", 42);
///
/// assert_eq!(user_tag, tag!("user:Bob"));
/// assert_eq!(machine_tag, tag!("machine:thing-42"));
/// ```
///
/// This will never panic because the initial tag is already proven to be a valid tag.
impl<T: Into<String>> Add<T> for Tag {
    type Output = Tag;
    fn add(self, rhs: T) -> Self::Output {
        Tag::from_str(&*(self.0.to_string() + rhs.into().as_str())).unwrap()
    }
}

/// A set of tags in canonical iteration order
///
/// All constructors and serialization ensure that tags appear only once and in string sort order.
#[derive(DagCbor, Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(from = "Vec<Tag>")]
#[ipld(repr = "value")]
pub struct TagSet(Vec<Tag>);

impl From<Vec<Tag>> for TagSet {
    fn from(mut v: Vec<Tag>) -> Self {
        v.sort_unstable();
        v.dedup();
        Self(v)
    }
}

impl From<&[Tag]> for TagSet {
    fn from(v: &[Tag]) -> Self {
        let v = Vec::from(v);
        Self::from(v)
    }
}

impl From<BTreeSet<Tag>> for TagSet {
    fn from(v: BTreeSet<Tag>) -> Self {
        Self(v.into_iter().collect())
    }
}

impl From<&BTreeSet<Tag>> for TagSet {
    fn from(v: &BTreeSet<Tag>) -> Self {
        Self(v.iter().cloned().collect())
    }
}

impl FromIterator<Tag> for TagSet {
    fn from_iter<T: IntoIterator<Item = Tag>>(iter: T) -> Self {
        let v = iter.into_iter().collect::<Vec<_>>();
        Self::from(v)
    }
}

impl IntoIterator for TagSet {
    type Item = Tag;

    type IntoIter = std::vec::IntoIter<Tag>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a TagSet {
    type Item = &'a Tag;

    type IntoIter = std::slice::Iter<'a, Tag>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl AsRef<[Tag]> for TagSet {
    fn as_ref(&self) -> &[Tag] {
        &self.0
    }
}

impl Default for TagSet {
    fn default() -> Self {
        Self::empty()
    }
}

impl TagSet {
    pub fn empty() -> Self {
        Self(Vec::new())
    }

    pub fn insert(&mut self, tag: Tag) {
        if let Err(idx) = self.0.binary_search(&tag) {
            self.0.insert(idx, tag);
        }
    }

    pub fn remove(&mut self, tag: &Tag) {
        if let Ok(idx) = self.0.binary_search(tag) {
            self.0.remove(idx);
        }
    }

    pub fn contains(&self, tag: &Tag) -> bool {
        self.0.binary_search(tag).is_ok()
    }

    pub fn iter(&self) -> impl Iterator<Item = Tag> + '_ {
        self.0.iter().cloned()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, idx: usize) -> Option<Tag> {
        self.0.get(idx).cloned()
    }

    pub fn find(&self, tag: &Tag) -> Option<usize> {
        self.0.binary_search(tag).ok()
    }

    pub fn union(&self, rhs: &TagSet) -> Self {
        let mut v = Vec::with_capacity(self.len() + rhs.len());
        let mut left = self.iter();
        let mut right = rhs.iter();
        let mut ll = left.next();
        let mut rr = right.next();
        loop {
            match (ll, rr) {
                (Some(l), Some(r)) => match l.cmp(&r) {
                    std::cmp::Ordering::Less => {
                        v.push(l);
                        ll = left.next();
                        rr = Some(r);
                    }
                    std::cmp::Ordering::Equal => {
                        v.push(l);
                        ll = left.next();
                        rr = right.next();
                    }
                    std::cmp::Ordering::Greater => {
                        v.push(r);
                        ll = Some(l);
                        rr = right.next();
                    }
                },
                (Some(l), None) => {
                    v.push(l);
                    v.extend(left);
                    break;
                }
                (None, Some(r)) => {
                    v.push(r);
                    v.extend(right);
                    break;
                }
                _ => break,
            }
        }
        TagSet(v)
    }

    pub fn intersection(&self, rhs: &TagSet) -> Self {
        self.iter().filter(|tag| rhs.contains(tag)).collect()
    }

    pub fn difference(&self, rhs: &TagSet) -> Self {
        self.iter().filter(|tag| !rhs.contains(tag)).collect()
    }

    pub fn is_subset(&self, rhs: &TagSet) -> bool {
        self.iter().all(|tag| rhs.contains(&tag))
    }

    pub fn into_inner(self) -> Vec<Tag> {
        self.0
    }
}

impl Add for &TagSet {
    type Output = TagSet;
    fn add(self, rhs: Self) -> Self::Output {
        self.union(&rhs)
    }
}

impl Add for TagSet {
    type Output = TagSet;
    fn add(self, rhs: Self) -> Self::Output {
        self.union(&rhs)
    }
}

impl Add<Tag> for TagSet {
    type Output = TagSet;
    fn add(mut self, rhs: Tag) -> Self::Output {
        self.insert(rhs);
        self
    }
}

impl Add<Tag> for &TagSet {
    type Output = TagSet;
    fn add(self, rhs: Tag) -> Self::Output {
        let mut ret = self.clone();
        ret.insert(rhs);
        ret
    }
}

impl Add<&Tag> for TagSet {
    type Output = TagSet;
    fn add(mut self, rhs: &Tag) -> Self::Output {
        self.insert(rhs.clone());
        self
    }
}

impl Add<&Tag> for &TagSet {
    type Output = TagSet;
    fn add(self, rhs: &Tag) -> Self::Output {
        let mut ret = self.clone();
        ret.insert(rhs.clone());
        ret
    }
}

impl AddAssign<Tag> for TagSet {
    fn add_assign(&mut self, rhs: Tag) {
        self.insert(rhs)
    }
}

impl AddAssign<TagSet> for TagSet {
    fn add_assign(&mut self, rhs: TagSet) {
        for tag in rhs.iter() {
            self.insert(tag)
        }
    }
}

impl SubAssign<&Tag> for TagSet {
    fn sub_assign(&mut self, rhs: &Tag) {
        self.remove(rhs)
    }
}

impl SubAssign<&TagSet> for TagSet {
    fn sub_assign(&mut self, rhs: &TagSet) {
        self.0.retain(|tag| !rhs.contains(tag))
    }
}

impl BitAndAssign<&TagSet> for TagSet {
    fn bitand_assign(&mut self, rhs: &TagSet) {
        self.0.retain(|tag| rhs.contains(tag))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{quickcheck, TestResult};

    #[test]
    fn make_tags() {
        let mut tags = BTreeSet::new();
        tags.insert(tag!("a"));
        tags.insert(tag!("a:b"));
        assert_eq!(tags!("a", "a:b"), TagSet::from(tags));
    }

    #[test]
    fn tagset_is_subset() {
        assert!(!tags!("a").is_subset(&tags!()));
        assert!(!tags!("a", "b").is_subset(&tags!()));
        assert!(!tags!("a", "b").is_subset(&tags!("a")));
        assert!(!tags!("a", "b").is_subset(&tags!("c")));

        assert!(tags!().is_subset(&tags!()));
        assert!(tags!().is_subset(&tags!("a")));
        assert!(tags!("a").is_subset(&tags!("a")));
        assert!(tags!("a").is_subset(&tags!("a", "b")));
        assert!(tags!("a", "b").is_subset(&tags!("a", "b")));
    }

    quickcheck! {
        fn canonicalise(s: String) -> TestResult {
            if s.is_empty() {
                TestResult::discard()
            } else {
                TestResult::from_bool(Tag::from_str(&*s).unwrap().to_string() == s.nfc().collect::<String>())
            }
        }

        fn canonicalise_serde(s: String) -> TestResult {
            if s.is_empty() {
                TestResult::discard()
            } else {
                let bytes = serde_json::to_vec(&s).unwrap();
                let t: Tag = serde_json::from_slice(&*bytes).unwrap();
                TestResult::from_bool(t.to_string() == s.nfc().collect::<String>())
            }
        }
    }

    #[test]
    fn tagset_is_set() {
        let a = tag!("a");
        let b = tag!("b");
        let c = tag!("c");

        assert_eq!(
            tags!("c", "b", "c", "a")
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
        assert_eq!(tags!("a", "b", "c", "b"), tags!("c", "b", "a"));
        assert_eq!(tags!("a", "b") + tags!("b", "c"), tags!("a", "b", "c"));
        assert_eq!(tags!("a", "c") + &b, tags!("a", "b", "c"));

        let mut t = TagSet::empty();
        t.insert(a.clone());
        assert!(t.contains(&a));
        assert!(!t.contains(&c));
        t.insert(b.clone());
        t.insert(c.clone());
        t.remove(&a);
        assert_eq!(t, tags!("c", "b"));

        assert_eq!(vec![a, b, c].into_iter().collect::<TagSet>(), tags!("a", "b", "c"));
    }
}
