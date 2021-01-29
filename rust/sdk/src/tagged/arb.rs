use super::{NodeId, StreamId, StreamNr};
use quickcheck::{Arbitrary, Gen};

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
