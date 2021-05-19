use crate::tag_index::IndexSet;
use actyxos_sdk::{LamportTimestamp, TagSet, Timestamp};
use banyan::{
    index::{BranchIndex, CompactSeq, LeafIndex, Summarizable},
    query::Query,
    Tree, TreeTypes,
};
use libipld::{
    cbor::DagCborCodec,
    codec::{Decode, Encode},
    multihash::{Code, Multihash, MultihashDigest},
    Cid, DagCbor,
};
use range_collections::RangeSet;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ord,
    convert::{TryFrom, TryInto},
    fmt,
    io::{Read, Seek, Write},
    iter::FromIterator,
    ops::{Range, RangeFrom, RangeTo},
    str::FromStr,
};

use crate::TagIndex;

pub type AxTree = Tree<AxTrees>;

const MAX_TAGSET_SIZE: usize = 4096;

/// An inclusive range, not called RangeInclusive in order to not mix it up with the stdlib type
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AxRange<T> {
    pub min: T,
    pub max: T,
}

impl<T: Copy> From<T> for AxRange<T> {
    fn from(value: T) -> Self {
        Self { min: value, max: value }
    }
}

impl<T> AxRange<T> {
    pub fn new(min: T, max: T) -> Self {
        Self { min, max }
    }
}

impl<A: smallvec::Array<Item = LamportTimestamp>> From<AxRange<LamportTimestamp>> for RangeSet<LamportTimestamp, A> {
    fn from(r: AxRange<LamportTimestamp>) -> Self {
        RangeSet::from(r.min..(r.max + 1))
    }
}

impl<A: smallvec::Array<Item = Timestamp>> From<AxRange<Timestamp>> for RangeSet<Timestamp, A> {
    fn from(r: AxRange<Timestamp>) -> Self {
        RangeSet::from(r.min..(r.max + 1))
    }
}

/// A single key. This represents the queryable part of an event
///
/// Typically you deal not with individual keys but with sequences of keys. See [AxKeySeq]
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct AxKey {
    pub(crate) tags: TagSet,
    pub(crate) time: Timestamp,
    pub(crate) lamport: LamportTimestamp,
}

impl AxKey {
    pub fn new(tags: TagSet, lamport: impl Into<LamportTimestamp>, time: impl Into<Timestamp>) -> Self {
        Self {
            tags,
            lamport: lamport.into(),
            time: time.into(),
        }
    }

    pub fn tags(&self) -> &TagSet {
        &self.tags
    }

    pub fn time(&self) -> Timestamp {
        self.time
    }

    pub fn lamport(&self) -> LamportTimestamp {
        self.lamport
    }

    pub fn into_tags(self) -> TagSet {
        self.tags
    }
}

/// The in memory representation of a sequence of ax keys
///
/// This is optimized for fast querying
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AxKeySeqIo", into = "AxKeySeqIo")]
pub struct AxKeySeq {
    tags: TagIndex,
    lamport: Vec<LamportTimestamp>,
    time: Vec<Timestamp>,
}

impl AxKeySeq {
    pub fn lamport_range(&self) -> AxRange<LamportTimestamp> {
        // we could assume that lamports are ordered, but let's play it safe for now
        let min_lamport = self.lamport.iter().min().unwrap();
        let max_lamport = self.lamport.iter().max().unwrap();
        AxRange::new(*min_lamport, *max_lamport)
    }

    pub fn time_range(&self) -> AxRange<Timestamp> {
        // we could assume that lamports are ordered, but let's play it safe for now
        let min_time = self.time.iter().min().unwrap();
        let max_time = self.time.iter().max().unwrap();
        AxRange::new(*min_time, *max_time)
    }
}

impl Encode<DagCborCodec> for AxKeySeq {
    fn encode<W: Write>(&self, c: DagCborCodec, w: &mut W) -> anyhow::Result<()> {
        AxKeySeqIo::from(self.clone()).encode(c, w)
    }
}

impl Decode<DagCborCodec> for AxKeySeq {
    fn decode<R: Read + Seek>(c: DagCborCodec, r: &mut R) -> anyhow::Result<Self> {
        let t: AxKeySeqIo = Decode::decode(c, r)?;
        t.try_into()
    }
}

/// The IO representation of a sequence of ax keys
///
/// This shuffles the data around a little bit so that once serialized via CBOR,
/// it can be better compressed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, DagCbor)]
struct AxKeySeqIo {
    tags: TagIndex,
    time: Vec<u64>,
    lamport: Vec<u64>,
}

impl From<AxKeySeq> for AxKeySeqIo {
    fn from(value: AxKeySeq) -> Self {
        // some data massaging to make life easier for zstd
        let mut lamport: Vec<u64> = Vec::with_capacity(value.len());
        for i in 0..value.len() {
            lamport.push(value.lamport[i].into());
        }
        let mut time: Vec<u64> = Vec::with_capacity(value.len());
        for i in 0..value.len() {
            time.push(value.time[i].into());
        }
        delta_encode(&mut time);
        delta_encode(&mut lamport);
        Self {
            tags: value.tags,
            time,
            lamport,
        }
    }
}

impl TryFrom<AxKeySeqIo> for AxKeySeq {
    type Error = anyhow::Error;
    fn try_from(mut value: AxKeySeqIo) -> Result<Self, Self::Error> {
        // reject unexpected blocks
        let n = value.tags.elements.len();
        if n == 0 {
            anyhow::bail!("must not be empty");
        }
        if value.time.len() != n {
            anyhow::bail!("time has wrong size");
        }
        if value.lamport.len() != n {
            anyhow::bail!("lamport has wrong size");
        }
        // reverse the data massaging
        delta_decode(&mut value.time);
        delta_decode(&mut value.lamport);
        let mut lamport: Vec<LamportTimestamp> = Vec::with_capacity(n);
        let mut time: Vec<Timestamp> = Vec::with_capacity(n);
        for i in 0..n {
            lamport.push(LamportTimestamp::new(value.lamport[i]));
            time.push(Timestamp::new(value.time[i]));
        }
        Ok(Self {
            tags: value.tags,
            lamport,
            time,
        })
    }
}

impl FromIterator<AxKey> for AxKeySeq {
    fn from_iter<I: IntoIterator<Item = AxKey>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;
        let mut tags = Vec::with_capacity(capacity);
        let mut lamport = Vec::with_capacity(capacity);
        let mut time = Vec::with_capacity(capacity);
        for key in iter {
            tags.push(key.tags);
            lamport.push(key.lamport);
            time.push(key.time);
        }
        let tags = TagIndex::from_elements(&tags);
        Self { tags, lamport, time }
    }
}

impl CompactSeq for AxKeySeq {
    type Item = AxKey;

    fn len(&self) -> usize {
        self.lamport.len()
    }

    fn get(&self, index: usize) -> Option<Self::Item> {
        self.tags.get(index).map(|tags| AxKey {
            tags,
            time: self.time[index],
            lamport: self.lamport[index],
        })
    }
}

impl Summarizable<AxSummary> for AxKeySeq {
    fn summarize(&self) -> AxSummary {
        let min_time = self.time.iter().min().unwrap();
        let max_time = self.time.iter().max().unwrap();
        // we could assume that lamports are ordered, but let's play it safe for now
        let min_lamport = self.lamport.iter().min().unwrap();
        let max_lamport = self.lamport.iter().max().unwrap();
        AxSummary {
            tags: TagsSummary::from(&self.tags.tags),
            time: AxRange::new(*min_time, *max_time),
            lamport: AxRange::new(*min_lamport, *max_lamport),
        }
    }
}

fn tags_too_large(tags: &TagSet) -> bool {
    let size: usize = tags.iter().map(|tag| tag.len() + 4).sum();
    size > MAX_TAGSET_SIZE
}

/// A summary of all tags in a tree.
///
/// In many cases, this can be just the complete set of tags
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TagsSummary {
    /// The complete set of tags in the tree
    Complete(TagSet),
    // Bloom(BloomFilter),
    /// No restriction on the tags in the tree
    Unrestricted,
}

impl TagsSummary {
    fn into_tags(self) -> Option<TagSet> {
        if let Self::Complete(tags) = self {
            Some(tags)
        } else {
            None
        }
    }
}

impl Default for TagsSummary {
    fn default() -> Self {
        Self::Complete(TagSet::empty())
    }
}

impl From<TagSet> for TagsSummary {
    fn from(tags: TagSet) -> Self {
        if !tags_too_large(&tags) {
            Self::Complete(tags)
        } else {
            Self::Unrestricted
        }
    }
}

impl From<&TagSet> for TagsSummary {
    fn from(tags: &TagSet) -> Self {
        if !tags_too_large(&tags) {
            Self::Complete(tags.clone())
        } else {
            Self::Unrestricted
        }
    }
}

impl FromIterator<TagsSummary> for TagsSummary {
    fn from_iter<T: IntoIterator<Item = TagsSummary>>(iter: T) -> Self {
        iter.into_iter().fold(TagsSummary::default(), |summary, item| {
            if let (TagsSummary::Complete(mut a), TagsSummary::Complete(b)) = (summary, item) {
                a += b;
                TagsSummary::from(a)
            } else {
                TagsSummary::Unrestricted
            }
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, DagCbor)]
pub enum TagsSummaries {
    /// The complete set of tags in the tree
    Complete(TagIndex),
    // Bloom(BloomFilters),
    /// No restriction on the tags in the tree
    Unrestricted,
}

impl Default for TagsSummaries {
    fn default() -> Self {
        Self::Complete(TagIndex::default())
    }
}

impl TagsSummaries {
    fn get(&self, offset: usize) -> Option<TagsSummary> {
        match self {
            Self::Complete(index) => index.get(offset).map(TagsSummary::Complete),
            Self::Unrestricted => Some(TagsSummary::Unrestricted),
        }
    }
}

impl Summarizable<TagsSummary> for TagsSummaries {
    fn summarize(&self) -> TagsSummary {
        match self {
            Self::Complete(tags) => TagsSummary::Complete(tags.tags.clone()),
            Self::Unrestricted => TagsSummary::Unrestricted,
        }
    }
}

impl FromIterator<TagsSummary> for TagsSummaries {
    fn from_iter<T: IntoIterator<Item = TagsSummary>>(iter: T) -> Self {
        let tags = iter.into_iter().map(|x| x.into_tags()).collect::<Option<Vec<TagSet>>>();
        tags.map(|tags| TagsSummaries::from(TagIndex::from_elements(&tags)))
            .unwrap_or(Self::Unrestricted)
    }
}

impl From<TagIndex> for TagsSummaries {
    fn from(index: TagIndex) -> Self {
        if !tags_too_large(&index.tags) {
            Self::Complete(index)
        } else {
            Self::Unrestricted
        }
    }
}

/// A single key. This represents the queryable part of an event
///
/// Typically you deal not with individual keys but with sequences of keys. See [AxKeySeq]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AxSummary {
    pub(crate) tags: TagsSummary,
    pub(crate) time: AxRange<Timestamp>,
    pub(crate) lamport: AxRange<LamportTimestamp>,
}

impl AxSummary {
    /// New key, for a single or multiple events
    pub fn new(tags: TagsSummary, lamport: AxRange<LamportTimestamp>, time: AxRange<Timestamp>) -> Self {
        Self { tags, lamport, time }
    }
}

/// The in memory representation of a sequence of ax summaries
///
/// This is optimized for fast querying
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AxSummarySeqIo", into = "AxSummarySeqIo")]
pub struct AxSummarySeq {
    tags: TagsSummaries,
    lamport: Vec<AxRange<LamportTimestamp>>,
    time: Vec<AxRange<Timestamp>>,
}

impl Encode<DagCborCodec> for AxSummarySeq {
    fn encode<W: Write>(&self, c: DagCborCodec, w: &mut W) -> anyhow::Result<()> {
        AxSummarySeqIo::from(self.clone()).encode(c, w)
    }
}

impl Decode<DagCborCodec> for AxSummarySeq {
    fn decode<R: Read + Seek>(c: DagCborCodec, r: &mut R) -> anyhow::Result<Self> {
        let t: AxSummarySeqIo = Decode::decode(c, r)?;
        t.try_into()
    }
}

impl AxSummarySeq {
    pub fn lamport_range(&self) -> AxRange<LamportTimestamp> {
        // we could assume that lamports are ordered, but let's play it safe for now
        let min_lamport = self.lamport.iter().map(|x| x.min).min().unwrap();
        let max_lamport = self.lamport.iter().map(|x| x.max).max().unwrap();
        AxRange::new(min_lamport, max_lamport)
    }

    pub fn time_range(&self) -> AxRange<Timestamp> {
        // we could assume that lamports are ordered, but let's play it safe for now
        let min_time = self.time.iter().map(|x| x.min).min().unwrap();
        let max_time = self.time.iter().map(|x| x.max).max().unwrap();
        AxRange::new(min_time, max_time)
    }
}

/// The IO representation of a sequence of ax keys
///
/// This shuffles the data around a little bit so that once serialized via CBOR,
/// it can be better compressed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, DagCbor)]
struct AxSummarySeqIo {
    tags: TagsSummaries,
    time: Vec<u64>,
    lamport: Vec<u64>,
}

fn delta_encode(data: &mut [u64]) {
    for i in (1..data.len()).rev() {
        data[i] = data[i].wrapping_sub(data[i - 1]);
    }
}

fn delta_decode(data: &mut [u64]) {
    for i in 1..data.len() {
        data[i] = data[i].wrapping_add(data[i - 1]);
    }
}

impl From<AxSummarySeq> for AxSummarySeqIo {
    fn from(value: AxSummarySeq) -> Self {
        // some data massaging to make life easier for zstd
        let mut lamport: Vec<u64> = Vec::with_capacity(value.len() * 2);
        for i in 0..value.len() {
            lamport.push(value.lamport[i].min.into());
            lamport.push(value.lamport[i].max.into());
        }
        let mut time: Vec<u64> = Vec::with_capacity(value.len() * 2);
        for i in 0..value.len() {
            time.push(value.time[i].min.into());
            time.push(value.time[i].max.into());
        }
        delta_encode(&mut time);
        delta_encode(&mut lamport);
        Self {
            tags: value.tags,
            time,
            lamport,
        }
    }
}

impl TryFrom<AxSummarySeqIo> for AxSummarySeq {
    type Error = anyhow::Error;
    fn try_from(mut value: AxSummarySeqIo) -> Result<Self, Self::Error> {
        // reject unexpected blocks
        let n = value.time.len() / 2;
        if n == 0 {
            anyhow::bail!("must not be empty");
        }
        if n * 2 != value.time.len() {
            anyhow::bail!("must not be odd length");
        }
        if value.lamport.len() != value.time.len() {
            anyhow::bail!("lamport has wrong size");
        }
        // reverse the data massaging
        delta_decode(&mut value.time);
        delta_decode(&mut value.lamport);
        let mut lamport: Vec<AxRange<LamportTimestamp>> = Vec::with_capacity(n);
        let mut time: Vec<AxRange<Timestamp>> = Vec::with_capacity(n);
        for i in 0..n {
            let min = LamportTimestamp::new(value.lamport[i * 2]);
            let max = LamportTimestamp::new(value.lamport[i * 2 + 1]);
            lamport.push(AxRange::new(min, max));

            let min = Timestamp::new(value.time[i * 2]);
            let max = Timestamp::new(value.time[i * 2 + 1]);
            time.push(AxRange::new(min, max));
        }
        Ok(Self {
            tags: value.tags,
            lamport,
            time,
        })
    }
}

impl FromIterator<AxSummary> for AxSummarySeq {
    fn from_iter<I: IntoIterator<Item = AxSummary>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;
        let mut tags = Vec::with_capacity(capacity);
        let mut lamport = Vec::with_capacity(capacity);
        let mut time = Vec::with_capacity(capacity);
        for key in iter {
            tags.push(key.tags);
            lamport.push(key.lamport);
            time.push(key.time);
        }
        let tags = tags.into_iter().collect::<TagsSummaries>();
        Self { tags, lamport, time }
    }
}

impl CompactSeq for AxSummarySeq {
    type Item = AxSummary;

    fn len(&self) -> usize {
        self.lamport.len()
    }

    fn get(&self, index: usize) -> Option<Self::Item> {
        self.tags.get(index).map(|tags| AxSummary {
            tags,
            time: self.time[index],
            lamport: self.lamport[index],
        })
    }
}

impl Summarizable<AxSummary> for AxSummarySeq {
    fn summarize(&self) -> AxSummary {
        let min_time = self.time.iter().map(|x| x.min).min().unwrap();
        let max_time = self.time.iter().map(|x| x.max).max().unwrap();
        // we could assume that lamports are ordered, but let's play it safe for now
        let min_lamport = self.lamport.iter().map(|x| x.min).min().unwrap();
        let max_lamport = self.lamport.iter().map(|x| x.max).max().unwrap();
        AxSummary {
            tags: self.tags.summarize(),
            time: AxRange::new(min_time, max_time),
            lamport: AxRange::new(min_lamport, max_lamport),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AxTrees;

impl TreeTypes for AxTrees {
    type Key = AxKey;
    type KeySeq = AxKeySeq;
    type Summary = AxSummary;
    type SummarySeq = AxSummarySeq;
    type Link = Sha256Digest;
}

#[derive(Debug, Clone)]
pub struct LamportQuery(RangeSet<LamportTimestamp>);

impl From<Range<LamportTimestamp>> for LamportQuery {
    fn from(value: Range<LamportTimestamp>) -> Self {
        Self(value.into())
    }
}

impl Query<AxTrees> for LamportQuery {
    fn intersecting(&self, _offset: u64, index: &BranchIndex<AxTrees>, matching: &mut [bool]) {
        let lamport = &index.summaries.lamport;
        for i in 0..lamport.len().min(matching.len()) {
            matching[i] = matching[i] && !self.0.is_disjoint(&lamport[i].clone().into());
        }
    }

    fn containing(&self, _offset: u64, index: &LeafIndex<AxTrees>, matching: &mut [bool]) {
        let lamport = &index.keys.lamport;
        for i in 0..lamport.len().min(matching.len()) {
            matching[i] = matching[i] && self.0.contains(&lamport[i]);
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeQuery(RangeSet<Timestamp>);

impl From<Range<Timestamp>> for TimeQuery {
    fn from(value: Range<Timestamp>) -> Self {
        Self(value.into())
    }
}

impl From<RangeSet<Timestamp>> for TimeQuery {
    fn from(value: RangeSet<Timestamp>) -> Self {
        Self(value)
    }
}

impl From<RangeFrom<Timestamp>> for TimeQuery {
    fn from(value: RangeFrom<Timestamp>) -> Self {
        Self(value.into())
    }
}

impl From<RangeTo<Timestamp>> for TimeQuery {
    fn from(value: RangeTo<Timestamp>) -> Self {
        Self(value.into())
    }
}

impl Query<AxTrees> for TimeQuery {
    fn intersecting(&self, _offset: u64, index: &BranchIndex<AxTrees>, matching: &mut [bool]) {
        let time = &index.summaries.time;
        for i in 0..time.len().min(matching.len()) {
            matching[i] = matching[i] && !self.0.is_disjoint(&time[i].clone().into());
        }
    }

    fn containing(&self, _offset: u64, index: &LeafIndex<AxTrees>, matching: &mut [bool]) {
        let time = &index.keys.time;
        for i in 0..time.len().min(matching.len()) {
            matching[i] = matching[i] && self.0.contains(&time[i]);
        }
    }
}

#[derive(Debug, Clone)]
pub struct OffsetQuery(RangeSet<u64>);

impl From<Range<u64>> for OffsetQuery {
    fn from(value: Range<u64>) -> Self {
        Self(value.into())
    }
}
impl From<RangeSet<u64>> for OffsetQuery {
    fn from(value: RangeSet<u64>) -> Self {
        Self(value)
    }
}

impl From<RangeFrom<u64>> for OffsetQuery {
    fn from(value: RangeFrom<u64>) -> Self {
        Self(value.into())
    }
}

impl From<RangeTo<u64>> for OffsetQuery {
    fn from(value: RangeTo<u64>) -> Self {
        Self(value.into())
    }
}

impl Query<AxTrees> for OffsetQuery {
    fn intersecting(&self, offset: u64, index: &BranchIndex<AxTrees>, matching: &mut [bool]) {
        let range = offset..offset + index.count;
        if self.0.is_disjoint(&range.into()) {
            for e in matching.iter_mut() {
                *e = false;
            }
        }
    }

    fn containing(&self, mut offset: u64, index: &LeafIndex<AxTrees>, matching: &mut [bool]) {
        for i in 0..index.keys.len().min(matching.len()) {
            matching[i] = matching[i] && self.0.contains(&offset);
            offset += 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct TagsQuery(Vec<TagSet>);

impl TagsQuery {
    pub fn new(dnf: Vec<TagSet>) -> Self {
        Self(dnf)
    }

    pub fn tags(&self) -> &[TagSet] {
        self.0.as_slice()
    }

    fn set_matching(&self, index: &TagIndex, matching: &mut [bool]) {
        if self.0.is_empty() {
            matching.fill(true);
            return;
        }

        // lookup all strings and translate them into indices.
        // if a single index does not match, the query can not match at all.
        let lookup = |s: &TagSet| -> Option<IndexSet> {
            s.iter()
                .map(|t| index.tags.find(&t).map(|x| x as u32))
                .collect::<Option<_>>()
        };
        // translate the query from strings to indices
        let query = self.0.iter().filter_map(lookup).collect::<Vec<_>>();
        // only look at bits that are currently set, set them to false if they do not match
        for i in 0..index.elements.len().min(matching.len()) {
            if matching[i] {
                matching[i] = query.iter().any(|x| x.is_subset(&index.elements[i]));
            }
        }
    }
}

impl Query<AxTrees> for TagsQuery {
    fn containing(&self, _offset: u64, index: &LeafIndex<AxTrees>, matching: &mut [bool]) {
        self.set_matching(&index.keys.tags, matching);
    }

    fn intersecting(&self, _offset: u64, index: &BranchIndex<AxTrees>, matching: &mut [bool]) {
        if let TagsSummaries::Complete(index) = &index.summaries.tags {
            self.set_matching(index, matching);
        }
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Sha256Digest([u8; 32]);

impl Sha256Digest {
    pub fn new(data: &[u8]) -> Self {
        let mh = Code::Sha2_256.digest(data);
        Sha256Digest(mh.digest().try_into().unwrap())
    }
}

impl Decode<DagCborCodec> for Sha256Digest {
    fn decode<R: Read + Seek>(c: DagCborCodec, r: &mut R) -> anyhow::Result<Self> {
        Self::try_from(libipld::Cid::decode(c, r)?)
    }
}
impl Encode<DagCborCodec> for Sha256Digest {
    fn encode<W: Write>(&self, c: DagCborCodec, w: &mut W) -> anyhow::Result<()> {
        libipld::Cid::encode(&Cid::from(*self), c, w)
    }
}

impl From<Sha256Digest> for Cid {
    fn from(value: Sha256Digest) -> Self {
        // https://github.com/multiformats/multicodec/blob/master/table.csv
        let mh = Multihash::wrap(0x12, &value.0).unwrap();
        Cid::new_v1(0x71, mh)
    }
}

impl TryFrom<Cid> for Sha256Digest {
    type Error = anyhow::Error;

    fn try_from(value: Cid) -> Result<Self, Self::Error> {
        anyhow::ensure!(value.codec() == 0x71, "Unexpected codec");
        anyhow::ensure!(value.hash().code() == 0x12, "Unexpected hash algorithm");
        let digest: [u8; 32] = value.hash().digest().try_into()?;
        Ok(Self(digest))
    }
}

impl FromStr for Sha256Digest {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let cid = Cid::from_str(s)?;
        cid.try_into()
    }
}

impl fmt::Display for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Cid::from(*self))
    }
}

impl fmt::Debug for Sha256Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", Cid::from(*self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyxos_sdk::{
        language::{TagAtom, TagExpr},
        tags, Tag,
    };
    use quickcheck::quickcheck;

    use crate::{axtrees::TagsQuery, TagSubscriptions};

    fn l(tag: &'static str) -> TagExpr {
        TagExpr::Atom(TagAtom::Tag(Tag::new(tag.to_owned()).unwrap()))
    }

    fn assert_match(index: &TagIndex, expr: &TagExpr, expected: Vec<bool>) {
        let tag_subscriptions = TagSubscriptions::from(expr);
        let query = TagsQuery::new(tag_subscriptions.as_tag_sets(true));
        let mut matching = vec![true; expected.len()];
        query.set_matching(index, &mut matching);
        assert_eq!(matching, expected);
    }

    #[test]
    fn test_matching_1() {
        let index = TagIndex::from_elements(&[tags!("a"), tags!("a", "b"), tags!("a"), tags!("a", "b")]);
        let expr = l("a") | l("b");
        assert_match(&index, &expr, vec![true, true, true, true]);
        let expr = l("a") & l("b");
        assert_match(&index, &expr, vec![false, true, false, true]);
        let expr = l("c") & l("d");
        assert_match(&index, &expr, vec![false, false, false, false]);
    }

    #[test]
    fn test_matching_2() {
        let index = TagIndex::from_elements(&[tags!("a", "b"), tags!("b", "c"), tags!("c", "a"), tags!("a", "b")]);
        let expr = l("a") | l("b") | l("c");
        assert_match(&index, &expr, vec![true, true, true, true]);
        let expr = l("a") & l("b");
        assert_match(&index, &expr, vec![true, false, false, true]);
        let expr = l("a") & l("b") & l("c");
        assert_match(&index, &expr, vec![false, false, false, false]);
    }

    quickcheck! {
            fn summaryseq_serde_roundtrip(ks: AxSummarySeq) -> bool {
                let cbor = serde_cbor::to_vec(&ks).expect("ks must be serializable");
                let ks1: AxSummarySeq = serde_cbor::from_slice(&cbor).expect("ks must be deserializable");
                ks == ks1
            }

            fn summaryseq_summarize(ks: AxSummarySeq) -> bool {
                let summary: AxSummary = ks.summarize();
                let elements = ks.to_vec();
                let mut lamport_min = LamportTimestamp::new(u64::max_value());
                let mut lamport_max = LamportTimestamp::new(u64::min_value());
                let mut time_min = Timestamp::new(u64::max_value());
                let mut time_max = Timestamp::new(u64::min_value());
                let tags = elements.iter().map(|e| e.tags.clone()).collect::<TagsSummary>();
                for e in elements {
                    lamport_min = lamport_min.min(e.lamport.min);
                    lamport_max = lamport_max.max(e.lamport.max);
                    time_min = time_min.min(e.time.min);
                    time_max = time_max.max(e.time.max);
                }
                let reference = AxSummary {
                    tags,
                    lamport: AxRange::new(lamport_min, lamport_max),
                    time: AxRange::new(time_min, time_max),
                };
                summary == reference
            }

        fn keyseq_serde_roundtrip(ks: AxKeySeq) -> bool {
            let cbor = serde_cbor::to_vec(&ks).expect("ks must be serializable");
            let ks1: AxKeySeq = serde_cbor::from_slice(&cbor).expect("ks must be deserializable");
            ks == ks1
        }

        fn keyseq_summarize(ks: AxKeySeq) -> bool {
            let summary: AxSummary = ks.summarize();
            let elements = ks.to_vec();
            let mut lamport_min = LamportTimestamp::new(u64::max_value());
            let mut lamport_max = LamportTimestamp::new(u64::min_value());
            let mut time_min = Timestamp::new(u64::max_value());
            let mut time_max = Timestamp::new(u64::min_value());
            let tags = elements.iter().map(|e| TagsSummary::from(&e.tags)).collect::<TagsSummary>();
            for e in elements {
                lamport_min = lamport_min.min(e.lamport);
                lamport_max = lamport_max.max(e.lamport);
                time_min = time_min.min(e.time);
                time_max = time_max.max(e.time);
            }
            let reference = AxSummary {
                tags,
                lamport: AxRange::new(lamport_min, lamport_max),
                time: AxRange::new(time_min, time_max),
            };
            summary == reference
        }
    }
}
