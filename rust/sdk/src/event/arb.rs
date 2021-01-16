use super::{scalars::MAX_SOURCEID_LENGTH, SourceId};
use crate::{tagged::StreamId, Offset, OffsetMap, OffsetOrMin};
use quickcheck::{Arbitrary, Gen};
use rand::Rng;
use std::{collections::BTreeMap, convert::TryFrom};

impl Arbitrary for SourceId {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let len = g.gen_range(1, MAX_SOURCEID_LENGTH);
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            s.push(g.gen_range(32u8, 127u8).into());
        }
        SourceId::try_from(s.as_str()).unwrap()
    }
}

impl Arbitrary for Offset {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let offset: u32 = Arbitrary::arbitrary(g);
        Self::from(offset)
    }
}

impl Arbitrary for OffsetOrMin {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        if g.gen_bool(0.8) {
            let offset: Offset = Arbitrary::arbitrary(g);
            Self::from(offset)
        } else {
            OffsetOrMin::MIN
        }
    }
}

impl Arbitrary for OffsetMap {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let inner: BTreeMap<StreamId, Offset> = Arbitrary::arbitrary(g);
        Self::from(inner)
    }
}
