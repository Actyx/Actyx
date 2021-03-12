use crate::{
    tagged::{NodeId, StreamId, StreamNr},
    Offset, OffsetMap, OffsetOrMin,
};
use quickcheck::{Arbitrary, Gen};
use std::collections::BTreeMap;

impl Arbitrary for NodeId {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut bytes = [0u8; 32];
        for b in &mut bytes {
            *b = u8::arbitrary(g);
        }
        NodeId(bytes)
    }
}

impl Arbitrary for StreamNr {
    fn arbitrary(g: &mut Gen) -> Self {
        u64::arbitrary(g).into()
    }
}

impl Arbitrary for StreamId {
    fn arbitrary(g: &mut Gen) -> Self {
        Self {
            node_id: NodeId::arbitrary(g),
            stream_nr: StreamNr::arbitrary(g),
        }
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
