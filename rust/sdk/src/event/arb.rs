use super::{scalars::MAX_SOURCEID_LENGTH, SourceId};
use crate::{tagged::StreamId, Offset, OffsetMap, OffsetOrMin};
use quickcheck::{Arbitrary, Gen};
use std::{
    collections::BTreeMap,
    convert::TryFrom,
    ops::{Add, Range, Rem, Sub},
};

fn gen_range<T>(g: &mut Gen, r: Range<T>) -> T
where
    T: Arbitrary + Add<Output = T> + Sub<Output = T> + Rem<Output = T> + Copy,
{
    T::arbitrary(g) % (r.end - r.start) + r.start
}

impl Arbitrary for SourceId {
    fn arbitrary(g: &mut Gen) -> Self {
        let len = gen_range(g, 1..MAX_SOURCEID_LENGTH);
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            s.push(gen_range(g, 32u8..127u8).into());
        }
        SourceId::try_from(s.as_str()).unwrap()
    }
}

impl Arbitrary for Offset {
    fn arbitrary(g: &mut Gen) -> Self {
        let offset: u32 = Arbitrary::arbitrary(g);
        Self::from(offset)
    }
}

impl Arbitrary for OffsetOrMin {
    fn arbitrary(g: &mut Gen) -> Self {
        if bool::arbitrary(g) {
            let offset: Offset = Arbitrary::arbitrary(g);
            Self::from(offset)
        } else {
            OffsetOrMin::MIN
        }
    }
}

impl Arbitrary for OffsetMap {
    fn arbitrary(g: &mut Gen) -> Self {
        let inner: BTreeMap<StreamId, Offset> = Arbitrary::arbitrary(g);
        Self::from(inner)
    }
}
