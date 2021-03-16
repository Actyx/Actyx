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
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    convert::TryFrom,
    fmt::{self, Debug},
    io::{Read, Seek, SeekFrom, Write},
    iter::FromIterator,
    ops::{Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, Sub, SubAssign},
};

use derive_more::{Display, From, Into};
use libipld::{
    cbor::DagCborCodec,
    codec::{Decode, Encode},
    DagCbor,
};
use num_traits::Bounded;
use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize,
};

use crate::event::{Event, EventKey};
use crate::scalars::StreamId;

/// Maximum possible offset
///
/// the max Offset needs to fit into an i64 and also needs to be losslessly converted into an f64
/// due to interop with braindead languages that do not have proper integers.
///
/// See https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MAX_SAFE_INTEGER
const MAX_SAFE_INT: i64 = 9_007_199_254_740_991;

/// Event offset within a [`SourceId`](struct.SourceId.html)’s stream or MIN value
///
/// The event offset is not a number, it rather is an identifier that can be compared
/// to other identifiers. There are 2^63 such values. The `incr` and `decr` functions
/// find the successor or predecessor, respectively. `incr` does not return an option
/// because for the use-case of naming events within a stream it is impossible to exhaust
/// the available set of values.
///
/// The MIN value is not a valid offset, it is sorted before [`Offset::ZERO`](struct.Offset.html#const.ZERO).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord, From, Into, Display)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct OffsetOrMin(#[serde(with = "i64_from_minus_one")] i64);

mod i64_from_minus_one {
    use super::*;
    use serde::Serializer;
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
        fn range<E: Error>(r: Result<i64, E>) -> Result<i64, E> {
            r.and_then(|i| {
                if i < -1 {
                    Err(E::custom(format!("number {} is below -1", i)))
                } else if i > MAX_SAFE_INT {
                    Err(E::custom(format!("number {} is too large", i)))
                } else {
                    Ok(i)
                }
            })
        }
        struct X;
        impl<'de> Visitor<'de> for X {
            type Value = i64;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string 'min'/'max' or integer")
            }
            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                match v {
                    "min" => Ok(-1),
                    "max" => Ok(MAX_SAFE_INT),
                    v => range(v.parse::<i64>().map_err(Error::custom)),
                }
            }
            fn visit_f64<E: Error>(self, v: f64) -> Result<Self::Value, E> {
                let i = v as i64;
                #[allow(clippy::float_cmp)]
                if i as f64 == v {
                    range(Ok(i))
                } else {
                    Err(E::custom("not an integer"))
                }
            }
            fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
                if v <= i64::max_value() as u64 {
                    range(Ok(v as i64))
                } else {
                    Err(E::custom("number too large"))
                }
            }
            fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
                range(Ok(v))
            }
        }
        d.deserialize_any(X)
    }
    pub fn serialize<S: Serializer>(t: &i64, s: S) -> Result<S::Ok, S::Error> {
        if *t < 0 {
            "min".serialize(s)
        } else {
            t.serialize(s)
        }
    }
}

impl OffsetOrMin {
    /// Zero offset, equal to [`Offset::ZERO`](struct.Offset.html#const.ZERO)
    pub const ZERO: OffsetOrMin = OffsetOrMin(0);

    /// Maximum possible offset
    ///
    /// the max Offset needs to fit into an i64 and also needs to be losslessly converted into an f64
    /// due to interop with braindead languages that do not have proper integers.
    ///
    /// See https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MAX_SAFE_INTEGER
    pub const MAX: OffsetOrMin = OffsetOrMin(MAX_SAFE_INT);

    /// Minimum value, predecessor of the ZERO offset
    pub const MIN: OffsetOrMin = OffsetOrMin(-1);

    /// This function shall only be used from tests to manufacture events where needed.
    ///
    /// It is intentionally hard to extract the wrapped number from this type because
    /// offsets do not support useful arithmetic operations.
    pub fn mk_test(o: u32) -> Self {
        Self(o.into())
    }

    /// Return the successor to this offset, where ZERO succeeds MIN
    pub fn succ(&self) -> Offset {
        if *self == Self::MAX {
            panic!("cannot increment OffsetOrMin({})", self)
        }
        Offset(self.0 + 1)
    }

    /// Return the predecessor to this offset
    pub fn pred(&self) -> Option<Self> {
        if self > &Self::MIN {
            Some(Self(self.0 - 1))
        } else {
            None
        }
    }
}

impl Default for OffsetOrMin {
    fn default() -> Self {
        Self::MIN
    }
}

impl From<Offset> for OffsetOrMin {
    fn from(o: Offset) -> Self {
        Self(o.0)
    }
}

impl From<u32> for OffsetOrMin {
    fn from(value: u32) -> Self {
        Self(value.into())
    }
}

impl PartialEq<Offset> for OffsetOrMin {
    fn eq(&self, other: &Offset) -> bool {
        OffsetOrMin::from(*other) == *self
    }
}

impl PartialOrd<Offset> for OffsetOrMin {
    fn partial_cmp(&self, other: &Offset) -> Option<Ordering> {
        self.partial_cmp(&OffsetOrMin::from(*other))
    }
}

impl Sub for OffsetOrMin {
    type Output = i64;
    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Add<u32> for OffsetOrMin {
    type Output = Self;
    fn add(self, rhs: u32) -> Self {
        if Self::MAX - self < i64::from(rhs) {
            panic!("cannot add {} to OffsetOrMin({})", rhs, self)
        }
        Self(self.0 + i64::from(rhs))
    }
}

impl Bounded for OffsetOrMin {
    fn min_value() -> Self {
        OffsetOrMin::MIN
    }
    fn max_value() -> Self {
        OffsetOrMin::MAX
    }
}

impl Encode<DagCborCodec> for OffsetOrMin {
    fn encode<W: Write>(&self, c: DagCborCodec, w: &mut W) -> anyhow::Result<()> {
        if self.0 < 0 {
            (-1i64).encode(c, w)
        } else {
            u64::try_from(self.0)?.encode(c, w)
        }
    }
}

impl Decode<DagCborCodec> for OffsetOrMin {
    fn decode<R: Read + Seek>(c: DagCborCodec, r: &mut R) -> anyhow::Result<Self> {
        let p = r.seek(SeekFrom::Current(0))?;
        Ok(if let Ok(value) = u64::decode(c, r) {
            let value = i64::try_from(value)?;
            anyhow::ensure!(value <= MAX_SAFE_INT);
            OffsetOrMin(value)
        } else {
            r.seek(SeekFrom::Start(p))?;
            let value = i64::decode(c, r)?;
            anyhow::ensure!(value == -1);
            OffsetOrMin::MIN
        })
    }
}

/// Event offset within a [`SourceId`](struct.SourceId.html)’s stream
///
/// The event offset is not a number, it rather is an identifier that can be compared
/// to other identifiers. There are 2^63 such values. The `incr` and `decr` functions
/// find the successor or predecessor, respectively. `incr` does not return an option
/// because for the use-case of naming events within a stream it is impossible to exhaust
/// the available set of values.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord, Display)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct Offset(#[serde(deserialize_with = "offset_i64")] i64);

impl From<Offset> for u64 {
    fn from(value: Offset) -> Self {
        // offset is guaranteed to be positive
        value.0 as u64
    }
}

impl From<u32> for Offset {
    fn from(value: u32) -> Offset {
        // An u32 is guaranteed to fit into an offset
        Self(value as i64)
    }
}

impl TryFrom<u64> for Offset {
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        if value > MAX_SAFE_INT as u64 {
            Err("number too large")
        } else {
            Ok(Offset(value as i64))
        }
    }
}

fn validate_offset(o: i64) -> Result<Offset, &'static str> {
    if o < 0 {
        Err("negative number")
    } else if o > MAX_SAFE_INT {
        Err("number too large")
    } else {
        Ok(Offset(o))
    }
}

fn offset_i64<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
    let o = i64::deserialize(d)?;
    validate_offset(o).map(|o| o - Offset::ZERO).map_err(D::Error::custom)
}

impl Offset {
    /// Minimum possible offset, also default value
    pub const ZERO: Offset = Offset(0);

    /// Maximum possible offset
    ///
    /// the max Offset needs to fit into an i64 and also needs to be losslessly converted into an f64
    /// due to interop with braindead languages that do not have proper integers.
    ///
    /// See https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number/MAX_SAFE_INTEGER
    pub const MAX: Offset = Offset(MAX_SAFE_INT);

    /// This function shall only be used from tests to manufacture events where needed.
    ///
    /// It is intentionally hard to extract the wrapped number from this type because
    /// offsets do not support useful arithmetic operations.
    pub fn mk_test(o: u32) -> Self {
        Self(o.into())
    }

    /// Fallible conversion from [`OffsetOrMin`](struct.OffsetOrMin.html)
    ///
    /// This returns `None` when presented with `OffsetOrMin::MIN`.
    pub fn from_offset_or_min(o: OffsetOrMin) -> Option<Self> {
        if o >= Self::ZERO {
            Some(Self(o.0))
        } else {
            None
        }
    }

    /// Return the successor to this offset
    pub fn succ(&self) -> Self {
        if *self == Self::MAX {
            panic!("cannot increment Offset({})", self)
        }
        Self(self.0 + 1)
    }

    /// Return the predecessor to this offset
    pub fn pred(&self) -> Option<Self> {
        if self > &Self::ZERO {
            Some(Self(self.0 - 1))
        } else {
            None
        }
    }

    /// Return the predecessor to this offset, possibly [`OffsetOrMin::MIN`](struct.OffsetOrMin.html#const.MIN)
    pub fn pred_or_min(&self) -> OffsetOrMin {
        OffsetOrMin(self.0 - 1)
    }
}

impl Default for Offset {
    fn default() -> Self {
        Self::ZERO
    }
}

impl PartialEq<OffsetOrMin> for Offset {
    fn eq(&self, other: &OffsetOrMin) -> bool {
        OffsetOrMin::from(*self) == *other
    }
}

impl PartialOrd<OffsetOrMin> for Offset {
    fn partial_cmp(&self, other: &OffsetOrMin) -> Option<Ordering> {
        OffsetOrMin::from(*self).partial_cmp(other)
    }
}

impl Sub for Offset {
    type Output = i64;
    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Add<u32> for Offset {
    type Output = Self;
    fn add(self, rhs: u32) -> Self {
        if Self::MAX - self < i64::from(rhs) {
            panic!("cannot add {} to Offset({})", rhs, self)
        }
        Self(self.0 + i64::from(rhs))
    }
}

impl Bounded for Offset {
    fn min_value() -> Self {
        Offset::ZERO
    }
    fn max_value() -> Self {
        Offset::MAX
    }
}

impl Encode<DagCborCodec> for Offset {
    fn encode<W: Write>(&self, c: DagCborCodec, w: &mut W) -> libipld::Result<()> {
        (self.0 as u64).encode(c, w)
    }
}

impl Decode<DagCborCodec> for Offset {
    fn decode<R: Read + Seek>(c: DagCborCodec, r: &mut R) -> libipld::Result<Self> {
        let raw = u64::decode(c, r)?;
        let validated = validate_offset(i64::try_from(raw)?).map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(validated)
    }
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use super::*;
    use rusqlite::{
        types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef},
        ToSql,
    };

    impl FromSql for Offset {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            value
                .as_i64()
                .and_then(|o| validate_offset(o).map_err(|_| FromSqlError::OutOfRange(o)))
        }
    }

    impl ToSql for Offset {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
            Ok(ToSqlOutput::from(self.0))
        }
    }
}

#[cfg(feature = "postgresql")]
mod postgresql {
    use super::*;
    use bytes::BytesMut;
    use postgres_types::{FromSql, IsNull, ToSql, Type};

    impl<'a> FromSql<'a> for Offset {
        fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
            i64::from_sql(ty, raw).and_then(|o| validate_offset(o).map_err(|e| e.into()))
        }
        fn accepts(ty: &Type) -> bool {
            <i64 as FromSql>::accepts(ty)
        }
    }

    impl ToSql for Offset {
        fn accepts(ty: &Type) -> bool
        where
            Self: Sized,
        {
            <i64 as ToSql>::accepts(ty)
        }
        fn to_sql_checked(
            &self,
            ty: &Type,
            out: &mut BytesMut,
        ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
            self.0.to_sql_checked(ty, out)
        }
        fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>>
        where
            Self: Sized,
        {
            self.0.to_sql(ty, out)
        }
    }
}

/// Multi-dimensional cursor for event streams: an `OffsetMap` describes the set of events
/// given by the event streams of each included source up to the associated [`Offset`](struct.Offset.html).
///
/// All stream delivery modes supported by the Event Service respect the order of offsets
/// of the events published by each single ActyxOS node. This order is consistent with the
/// Lamport timestamp order because both numbers are assigned to published events in strictly
/// monotonically increasing fashion, i.e. greater Offset implies greater Lamport timestamp
/// and vice versa.
///
/// > Note that if the `OffsetMap` contains offset 42 for SourceID `"abc"` it denotes that
/// events with offsets 0 through 42 (inclusive) are included within the `OffsetMap`.
///
/// A common usage pattern is to store the `OffsetMap` describing the events already consumed
/// from an event stream together with the computation results from processing those events
/// (preferably within the same database transaction, if applicable). When restarting the
/// process, this `OffsetMap` can be read and the stream can be resumed from where the process
/// left off previously.
///
/// ## Arithmetics
///
/// `OffsetMap` has a partial order: when the set of events described by one is a strict
/// subset of the set of events described by another, then one is said to be _smaller_ than
/// the other. It may be that one `OffsetMap` contains events that the other does not and vice
/// versa, in which case they are incomparable (`partial_cmp` will return `None`).
///
/// An event may be added into an `OffsetMap` to denote that from the event’s source all events
/// up to this one shall be included in the `OffsetMap`.
///
/// The difference of two offset maps yields the number of events contained within the first
/// but not within the second one (i.e. it counts the size of the difference set).
///
/// # Deserialization
///
/// An OffsetMap only contains valid offsets (non-negative numbers), but during deserialization
/// negative values are tolerated and ignored. This is to keep compatibility with previously
/// documented API endpoints.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize, DagCbor)]
#[serde(from = "BTreeMap<StreamId, OffsetOrMin>")]
pub struct OffsetMap(BTreeMap<StreamId, Offset>);

impl OffsetMap {
    /// The empty `OffsetMap` is equivalent to the beginning of time, it does not contain any
    /// event.
    pub fn empty() -> Self {
        Default::default()
    }

    /// Returns true if this `OffsetMap` does not contain any events
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Check whether the given Event’s offset and source ID are contained within this `OffsetMap`.
    pub fn contains<T>(&self, event: &Event<T>) -> bool {
        self.0.get(&event.key.stream).copied().unwrap_or_default() >= event.key.offset
    }

    /// Check whether the given stream contributes to the set of events in this OffsetMap
    pub fn contains_stream(&self, stream: &StreamId) -> bool {
        self.0.contains_key(stream)
    }

    /// Retrieve the offset stored for the given source
    ///
    /// The returned value is `OffsetOrMin::MIN` if nothing is stored for the given source.
    pub fn offset(&self, stream: impl Into<StreamId>) -> OffsetOrMin {
        self.get(stream).map(|o| o.into()).unwrap_or(OffsetOrMin::MIN)
    }

    /// Retrieves the offset stored for the given source
    pub fn get(&self, stream: impl Into<StreamId>) -> Option<Offset> {
        self.0.get(&stream.into()).cloned()
    }

    /// Counts the number of offsets spanned by this OffsetMap.
    pub fn size(&self) -> u64 {
        self - &OffsetMap::empty()
    }

    /// Merge the other OffsetMap into this one, taking the union of their event sets.
    pub fn union_with(&mut self, other: &OffsetMap) {
        for (k, v) in &other.0 {
            self.0.entry(*k).and_modify(|me| *me = (*me).max(*v)).or_insert(*v);
        }
    }

    /// Compute the union of two sets of events described by OffsetMaps
    pub fn union(&self, other: &OffsetMap) -> OffsetMap {
        let mut copy = self.clone();
        copy.union_with(other);
        copy
    }

    /// Compute the intersection of two sets of events described by OffsetMaps
    #[allow(clippy::needless_collect)]
    pub fn intersection_with(&mut self, other: &OffsetMap) {
        let keys = self.0.keys().cloned().collect::<Vec<_>>();
        for key in keys.into_iter() {
            let offset = other.offset(key).min(self.offset(key));
            if let Some(offset) = Offset::from_offset_or_min(offset) {
                self.0.insert(key, offset);
            } else {
                self.0.remove(&key);
            }
        }
    }

    /// Compute the intersection of two sets of events described by OffsetMaps
    pub fn intersection(&self, other: &OffsetMap) -> OffsetMap {
        let left = self.0.keys().collect::<BTreeSet<_>>();
        let right = other.0.keys().collect::<BTreeSet<_>>();
        let keys = left.intersection(&right);
        Self(
            keys.map(|key| {
                (
                    **key,
                    self.0
                        .get(key)
                        .copied()
                        .unwrap_or_default()
                        .min(other.0.get(key).copied().unwrap_or_default()),
                )
            })
            .collect(),
        )
    }

    pub fn into_inner(self) -> BTreeMap<StreamId, Offset> {
        self.0
    }

    /// An iterator over all streams that contribute events to this OffsetMap
    pub fn streams(&self) -> impl Iterator<Item = StreamId> + '_ {
        self.0.keys().copied()
    }

    /// An iterator over all streams that contribute events to this OffsetMap including their offset
    pub fn stream_iter(&self) -> impl Iterator<Item = (StreamId, Offset)> + '_ {
        self.0.iter().map(|(k, v)| (*k, *v))
    }

    /// Update entry for source if the given offset is larger than the stored one
    /// and return the previous offset for this source
    pub fn update(&mut self, stream: impl Into<StreamId>, offset: Offset) -> Option<OffsetOrMin> {
        let stream = stream.into();
        let previous = self.offset(stream);
        if offset > previous {
            self.0.insert(stream, offset);
            Some(previous)
        } else {
            None
        }
    }
}

impl PartialOrd for OffsetMap {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        let lhs = self;
        let mut lt = false;
        let mut eq = false;
        let mut gt = false;
        let mut cross = |a: &OffsetOrMin, b: &OffsetOrMin| -> bool {
            match Ord::cmp(a, b) {
                Ordering::Less => lt = true,
                Ordering::Equal => eq = true,
                Ordering::Greater => gt = true,
            }
            lt && gt
        };
        for (k, a) in &lhs.0 {
            let b = &rhs.offset(*k);
            if cross(&OffsetOrMin::from(*a), b) {
                return None;
            }
        }
        for (k, b) in &rhs.0 {
            let a = &lhs.offset(*k);
            if cross(a, &OffsetOrMin::from(*b)) {
                return None;
            }
        }
        if lt {
            Some(Ordering::Less)
        } else if gt {
            Some(Ordering::Greater)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl AsRef<BTreeMap<StreamId, Offset>> for OffsetMap {
    fn as_ref(&self) -> &BTreeMap<StreamId, Offset> {
        &self.0
    }
}

impl Default for OffsetMap {
    fn default() -> Self {
        OffsetMap(BTreeMap::new())
    }
}

impl From<BTreeMap<StreamId, Offset>> for OffsetMap {
    fn from(map: BTreeMap<StreamId, Offset>) -> Self {
        Self(map)
    }
}

impl From<BTreeMap<StreamId, OffsetOrMin>> for OffsetMap {
    fn from(map: BTreeMap<StreamId, OffsetOrMin>) -> Self {
        map.into_iter()
            .filter_map(|(s, o)| Offset::from_offset_or_min(o).map(|o| (s, o)))
            .collect()
    }
}

impl FromIterator<(StreamId, Offset)> for OffsetMap {
    fn from_iter<T: IntoIterator<Item = (StreamId, Offset)>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T> AddAssign<&Event<T>> for OffsetMap {
    fn add_assign(&mut self, other: &Event<T>) {
        let off = self.0.entry(other.key.stream).or_default();
        if *off < other.key.offset {
            *off = other.key.offset;
        }
    }
}

impl AddAssign<&EventKey> for OffsetMap {
    fn add_assign(&mut self, other: &EventKey) {
        let off = self.0.entry(other.stream).or_default();
        if *off < other.offset {
            *off = other.offset;
        }
    }
}

impl<T> SubAssign<&Event<T>> for OffsetMap {
    /// Ensure that the given event is no longer contained within this OffsetMap.
    fn sub_assign(&mut self, other: &Event<T>) {
        let off = self.0.entry(other.key.stream).or_default();
        if *off >= other.key.offset {
            if let Some(o) = other.key.offset.pred() {
                *off = o;
            } else {
                self.0.remove(&other.key.stream);
            }
        }
    }
}

impl SubAssign<&EventKey> for OffsetMap {
    /// Ensure that the given event is no longer contained within this OffsetMap.
    fn sub_assign(&mut self, other: &EventKey) {
        let off = self.0.entry(other.stream).or_default();
        if *off >= other.offset {
            if let Some(o) = other.offset.pred() {
                *off = o;
            } else {
                self.0.remove(&other.stream);
            }
        }
    }
}

impl Sub<OffsetMap> for OffsetMap {
    type Output = u64;
    fn sub(self, other: Self) -> u64 {
        &self - &other
    }
}

impl Sub<&OffsetMap> for &OffsetMap {
    type Output = u64;
    fn sub(self, other: &OffsetMap) -> u64 {
        let mut ret = 0;
        for (k, a) in &self.0 {
            let a: OffsetOrMin = (*a).into();
            let b = other.offset(*k);
            if a > b {
                ret += (a - b) as u64;
            }
        }
        ret
    }
}

impl BitAnd for OffsetMap {
    type Output = OffsetMap;
    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersection(&rhs)
    }
}

impl BitAnd for &OffsetMap {
    type Output = OffsetMap;
    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersection(rhs)
    }
}

impl BitAndAssign for OffsetMap {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = &*self & &rhs;
    }
}

impl BitOr for OffsetMap {
    type Output = OffsetMap;
    fn bitor(mut self, rhs: Self) -> Self::Output {
        self.union_with(&rhs);
        self
    }
}

impl BitOr for &OffsetMap {
    type Output = OffsetMap;
    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl BitOrAssign for OffsetMap {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = &*self | &rhs;
    }
}

#[cfg(test)]
mod tests {
    use libipld::{codec::assert_roundtrip, ipld};

    use super::*;
    use crate::{
        event::{Metadata, Payload},
        scalars::NodeId,
        tags,
        timestamp::{LamportTimestamp, Timestamp},
    };

    const NODE: &str = "uAQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA";

    fn stream_id(stream_nr: u64) -> StreamId {
        NodeId::try_from(NODE).unwrap().stream(stream_nr.into())
    }

    fn mk_event(stream_nr: u64, offset: u32) -> Event<Payload> {
        Event {
            key: EventKey {
                lamport: LamportTimestamp::new(1),
                stream: stream_id(stream_nr),
                offset: Offset::mk_test(offset),
            },
            meta: Metadata {
                timestamp: Timestamp::now(),
                tags: tags!("dummy"),
            },
            payload: Payload::default(),
        }
    }

    #[test]
    #[allow(clippy::eq_op)]
    pub fn must_calculate_offset_map() {
        let ev1 = &mk_event(1, 1);
        let ev2 = &mk_event(2, 2);
        let ev3 = &mk_event(3, 1);

        let empty = &OffsetMap::default();
        let mut map1 = empty.clone();
        map1 += ev1;
        let mut map2 = map1.clone();
        map2 += ev2;
        let mut map3 = map1.clone();
        map3 += ev3;

        assert_eq!(&map2 - &map2, 0);
        assert_eq!(&map2 - &map1, 3);
        assert_eq!(&map2 - empty, 5);

        assert!(map2.contains(ev1));
        assert!(map1.contains(ev1));
        assert!(map2.contains(ev2));
        assert!(!map1.contains(ev2));

        assert!(map1 > *empty);
        assert!(map1 <= map1);
        assert!(map1 >= map1);
        assert!(map3 > map1);
        assert!(map2 > map1);
        assert!(map2.partial_cmp(&map3).is_none());

        // also need to test the consuming Sub impl
        assert_eq!(map1 - map2, 0);
    }

    #[test]
    pub fn must_set_op() {
        let left = OffsetMap::from(
            [
                (stream_id(1), Offset::mk_test(1)),
                (stream_id(2), Offset::mk_test(2)),
                (stream_id(3), Offset::mk_test(3)),
                (stream_id(4), Offset::mk_test(4)),
            ]
            .iter()
            .copied()
            .collect::<BTreeMap<_, _>>(),
        );

        let right = OffsetMap::from(
            [
                (stream_id(2), Offset::mk_test(4)),
                (stream_id(3), Offset::mk_test(3)),
                (stream_id(4), Offset::mk_test(2)),
                (stream_id(5), Offset::mk_test(1)),
            ]
            .iter()
            .copied()
            .collect::<BTreeMap<_, _>>(),
        );

        let union = OffsetMap::from(
            [
                (stream_id(1), Offset::mk_test(1)),
                (stream_id(2), Offset::mk_test(4)),
                (stream_id(3), Offset::mk_test(3)),
                (stream_id(4), Offset::mk_test(4)),
                (stream_id(5), Offset::mk_test(1)),
            ]
            .iter()
            .copied()
            .collect::<BTreeMap<_, _>>(),
        );

        let intersection = OffsetMap::from(
            [
                (stream_id(2), Offset::mk_test(2)),
                (stream_id(3), Offset::mk_test(3)),
                (stream_id(4), Offset::mk_test(2)),
            ]
            .iter()
            .copied()
            .collect::<BTreeMap<_, _>>(),
        );

        assert_eq!(left.union(&right), union);
        assert_eq!(left.intersection(&right), intersection);
        assert_eq!(&left | &right, union);
        assert_eq!(left & right, intersection);
    }

    #[test]
    fn must_to_string() {
        assert_eq!(OffsetOrMin(12).to_string(), "12");
        assert_eq!(Offset::mk_test(3).to_string(), "3");
    }

    fn ser<T: Serialize>(v: T, expected: &str) {
        assert_eq!(serde_json::to_string(&v).unwrap(), expected);
    }

    fn de<'de, T: Deserialize<'de> + Debug + PartialEq>(from: &'de str, expected: T) {
        assert_eq!(serde_json::from_str::<T>(from).unwrap(), expected);
    }

    fn err<'de, T: Deserialize<'de> + Debug>(from: &'de str, msg: &str) {
        let s = serde_json::from_str::<T>(from).unwrap_err().to_string();
        assert!(s.contains(msg), "{} did not contain {}", s, msg);
    }

    #[test]
    fn must_serde_offset() {
        ser(Offset::ZERO, "0");
        ser(Offset::mk_test(1), "1");
        ser(Offset::MAX, "9007199254740991");

        de("0", Offset::ZERO);
        de("1", Offset::mk_test(1));
        de("9007199254740991", Offset::MAX);

        err::<Offset>("-1", "negative");
        err::<Offset>("-42", "negative");
        err::<Offset>("90071992547409911", "too large");
    }

    #[test]
    fn must_serde_offset_or_min() {
        ser(OffsetOrMin::MIN, "\"min\"");
        ser(OffsetOrMin::ZERO, "0");
        ser(OffsetOrMin::mk_test(42), "42");
        ser(OffsetOrMin::MAX, "9007199254740991");

        de("\"min\"", OffsetOrMin::MIN);
        de("-1", OffsetOrMin::MIN);
        de("\"-1\"", OffsetOrMin::MIN);
        de("0", OffsetOrMin::ZERO);
        de("1", OffsetOrMin::mk_test(1));
        de("9007199254740991", OffsetOrMin::MAX);
        de("\"9007199254740991\"", OffsetOrMin::MAX);
        de("\"max\"", OffsetOrMin::MAX);

        err::<OffsetOrMin>("-2", "below -1");
        err::<OffsetOrMin>("-20000000000000000000", "not an integer");
        err::<OffsetOrMin>("3.4", "not an integer");
        err::<OffsetOrMin>("90071992547409911", "too large");
        err::<OffsetOrMin>("\"-2\"", "below -1");
        err::<OffsetOrMin>("\"-20000000000000000000\"", "number too small");
        err::<OffsetOrMin>("\"3.4\"", "invalid digit");
        err::<OffsetOrMin>("\"90071992547409911\"", "too large");
    }

    #[test]
    fn must_serde_offset_map() {
        let mut map = OffsetMap::empty();
        ser(map.clone(), "{}");
        let stream = NodeId::try_from(NODE).unwrap().stream(0.into());
        map.update(stream, Offset::mk_test(12));
        ser(map.clone(), format!("{{\"{}\":12}}", stream).as_str());

        de("{}", OffsetMap::empty());
        de(format!("{{\"{}\":-1}}", stream).as_str(), OffsetMap::empty());
        de(format!("{{\"{}\":12}}", stream).as_str(), map);

        err::<OffsetMap>(format!("{{\"{}\":-11}}", stream).as_str(), "below -1");
    }

    #[test]
    fn offset_libipld() {
        // check offset roundtrip for obvious values
        assert_roundtrip(DagCborCodec, &Offset::from(0), &ipld!(0));
        assert_roundtrip(DagCborCodec, &Offset::from(1), &ipld!(1));
        assert_roundtrip(DagCborCodec, &Offset::MAX, &ipld!(MAX_SAFE_INT));
    }

    #[test]
    fn offset_or_min_libipld() {
        // check offsetormin roundtrip for obvious values
        assert_roundtrip(DagCborCodec, &OffsetOrMin::MIN, &ipld!(-1));
        assert_roundtrip(DagCborCodec, &OffsetOrMin::from(0u32), &ipld!(0));
        assert_roundtrip(DagCborCodec, &OffsetOrMin::from(1u32), &ipld!(1));
        assert_roundtrip(DagCborCodec, &OffsetOrMin::MAX, &ipld!(MAX_SAFE_INT));
    }
}
