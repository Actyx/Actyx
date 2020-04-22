/*
 * Copyright 2020 Actyx AG
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
use super::{Event, SourceId};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{AddAssign, Sub};

/// Each ActyxOS node marks the events it publishes with its source ID and assigns
/// a unique (consecutive) number to it: the Offset. The first event occupies offset zero.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
pub struct Offset(pub i64);

impl Default for Offset {
    fn default() -> Self {
        Self(-1)
    }
}

/// Multi-dimensional cursor for event streams: an OffsetMap describes the set of events
/// given by the event streams of each included source up to the associated [`Offset`](struct.Offset.html).
///
/// All stream delivery modes supported by the Event Service respect the order of offsets
/// of the events published by each single ActyxOS node. This order is consistent with the
/// Lamport timestamp order because both numbers are assigned to published events in strictly
/// monotonically increasing fashion, i.e. greater Offset implies greater Lamport timestamp
/// and vice versa.
///
/// > Note that if the OffsetMap contains Offset 42 for SourceID `"abc"` it denotes that
/// events with offsets 0 through 42 (inclusive) are included within the OffsetMap.
///
/// A common usage pattern is to store the OffsetMap describing the events already consumed
/// from an event stream together with the computation results from processing those events
/// (preferably within the same database transaction, if applicable). When restarting the
/// process, this OffsetMap can be read and the stream can be resumed from where the process
/// left off previously.
///
/// ## Arithmetics
///
/// OffsetMaps have a partial order: when the set of events described by one is a strict
/// subset of the set of events described by another, then one is said to be _smaller_ than
/// the other. It may be that one OffsetMap contains events that the other does not and vice
/// versa, in which case they are incomparable (`partial_cmp` will return `None`).
///
/// An event may be added into an OffsetMap to denote that from the event’s source all events
/// up to this one shall be included in the OffsetMap.
///
/// ```rust
/// # use actyxos_sdk::event::{Event, OffsetMap, Payload};
/// let mut offsets: OffsetMap = OffsetMap::empty();
/// let event: Event<Payload> = Event::mk_test("semantics", "name", "42").unwrap();
///
/// // keeping track of having seen this event:
/// offsets += &event;
/// assert!(offsets.contains(&event));
/// ```
///
/// The difference of two OffsetMaps yields the number of events contained within the first
/// but not within the second one (i.e. it counts the size of the difference set).
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct OffsetMap(HashMap<SourceId, Offset>);

impl OffsetMap {
    /// The empty OffsetMap is equivalent to the beginning of time, it does not contain any
    /// event.
    pub fn empty() -> Self {
        Default::default()
    }

    /// Check whether the given Event’s offset and source ID are contained within this OffsetMap.
    pub fn contains<T>(&self, event: &Event<T>) -> bool {
        self.0
            .get(&event.stream.source)
            .copied()
            .unwrap_or_default()
            >= event.offset
    }
}

impl PartialOrd for OffsetMap {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        let lhs = self;
        let mut lt = false;
        let mut eq = false;
        let mut gt = false;
        let mut cross = |a: &Offset, b: &Offset| -> bool {
            match Ord::cmp(a, b) {
                Ordering::Less => lt = true,
                Ordering::Equal => eq = true,
                Ordering::Greater => gt = true,
            }
            lt && gt
        };
        for (k, a) in &lhs.0 {
            let b = &rhs.0.get(k).copied().unwrap_or_default();
            if cross(a, b) {
                return None;
            }
        }
        for (k, b) in &rhs.0 {
            let a = &lhs.0.get(k).copied().unwrap_or_default();
            if cross(a, b) {
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

impl AsRef<HashMap<SourceId, Offset>> for OffsetMap {
    fn as_ref(&self) -> &HashMap<SourceId, Offset> {
        &self.0
    }
}

impl Default for OffsetMap {
    fn default() -> Self {
        OffsetMap(HashMap::new())
    }
}

impl From<HashMap<SourceId, Offset>> for OffsetMap {
    fn from(map: HashMap<SourceId, Offset>) -> Self {
        Self(map)
    }
}

impl<T> AddAssign<&Event<T>> for OffsetMap {
    fn add_assign(&mut self, other: &Event<T>) {
        let off = self.0.entry(other.stream.source).or_default();
        if *off < other.offset {
            *off = other.offset;
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
            let b = &other.0.get(k).copied().unwrap_or_default();
            if a > b {
                ret += (a.0 - b.0) as u64;
            }
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{
        FishName, LamportTimestamp, Payload, Semantics, SourceId, StreamInfo, TimeStamp,
    };
    use std::str::FromStr;

    fn mk_event(source: &str, offset: i64) -> Event<Payload> {
        Event {
            lamport: LamportTimestamp::new(1),
            stream: StreamInfo {
                semantics: Semantics::from("dummy"),
                name: FishName::from("dummy"),
                source: SourceId::from_str(source).unwrap(),
            },
            offset: Offset(offset),
            timestamp: TimeStamp::now(),
            payload: Payload::default(),
        }
    }

    #[test]
    #[allow(clippy::eq_op)]
    pub fn must_calculate_offset_map() {
        let ev1 = &mk_event("a", 1);
        let ev2 = &mk_event("b", 2);
        let ev3 = &mk_event("c", 1);

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
}
