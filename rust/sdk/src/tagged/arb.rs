use quickcheck::{Arbitrary, Gen};

use crate::event::SourceId;

use super::{NodeId, StreamId};

impl Arbitrary for NodeId {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut bytes = [0u8; 32];
        for b in &mut bytes {
            *b = u8::arbitrary(g);
        }
        NodeId(bytes)
    }
}

impl Arbitrary for StreamId {
    fn arbitrary(g: &mut Gen) -> Self {
        let stream_nr = u64::arbitrary(g);
        if stream_nr == 0 {
            SourceId::arbitrary(g).into()
        } else {
            Self {
                node_id: NodeId::arbitrary(g),
                stream_nr,
            }
        }
    }
}
