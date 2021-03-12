use std::convert::TryInto;

use super::Offset;
use actyxos_sdk::{tagged::StreamId, OffsetMap, OffsetOrMin};
use num_traits::Bounded;

#[derive(Clone, PartialOrd, PartialEq, Debug)]
/// Wrapper around an `OffsetMap` providing a default case
pub struct OffsetMapOrMax {
    /// either an offset map or a synthetic value that is larger than any offset map
    map: Option<OffsetMap>,
}

impl Bounded for OffsetMapOrMax {
    fn min_value() -> Self {
        OffsetMapOrMax {
            map: Some(OffsetMap::empty()),
        }
    }

    fn max_value() -> Self {
        OffsetMapOrMax { map: None }
    }
}

impl TryInto<OffsetMap> for OffsetMapOrMax {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<OffsetMap, Self::Error> {
        self.map.ok_or_else(|| anyhow::anyhow!("No offset map"))
    }
}

impl Default for OffsetMapOrMax {
    fn default() -> Self {
        Self::min_value()
    }
}

impl OffsetMapOrMax {
    pub fn from_entries(entries: &[(StreamId, OffsetOrMin)]) -> OffsetMapOrMax {
        let map = entries
            .iter()
            .filter_map(|(s, o)| Offset::from_offset_or_min(*o).map(|o| (*s, o)))
            .collect();
        OffsetMapOrMax { map: Some(map) }
    }

    pub fn offset(&self, stream_id: StreamId) -> OffsetOrMin {
        match &self.map {
            Some(map) => map.offset(stream_id),
            None => OffsetOrMin::MAX,
        }
    }

    pub fn get_default(&self) -> OffsetOrMin {
        match self.map {
            Some(_) => OffsetOrMin::MIN,
            None => OffsetOrMin::MAX,
        }
    }

    /// Takes the maximum of all entries among the two PsnMaps, persisting the maximum in the
    /// receiver side.
    pub fn max_with(&mut self, other: &OffsetMapOrMax) {
        match (&mut self.map, &other.map) {
            (Some(map), Some(other)) => map.union_with(other),
            (Some(_), None) => self.map = None,
            _ => (),
        }
    }

    /// Takes the minimum of all entries among the two PsnMaps, persisting the minimum in the
    /// receiver side.
    pub fn min_with(&mut self, other: &OffsetMapOrMax) {
        match (&mut self.map, &other.map) {
            (Some(map), Some(other)) => map.intersection_with(other),
            (None, Some(x)) => self.map = Some(x.clone()),
            _ => (),
        }
    }

    pub fn update(&mut self, src: StreamId, offset: OffsetOrMin) {
        if let (Some(map), Some(offset)) = (&mut self.map, Offset::from_offset_or_min(offset)) {
            map.update(src, offset);
        }
    }

    pub fn streams<'a>(&'a self) -> Box<dyn Iterator<Item = StreamId> + 'a> {
        match &self.map {
            Some(map) => Box::new(map.streams()),
            None => Box::new(std::iter::empty()),
        }
    }
}

impl From<OffsetMap> for OffsetMapOrMax {
    fn from(other: OffsetMap) -> OffsetMapOrMax {
        OffsetMapOrMax { map: Some(other) }
    }
}
