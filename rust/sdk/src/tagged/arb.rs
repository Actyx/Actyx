use quickcheck::Arbitrary;

use crate::event::SourceId;

use super::{NodeId, StreamId};

impl Arbitrary for NodeId {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
        let mut bytes = [0u8; 32];
        g.fill_bytes(&mut bytes[..]);
        NodeId(bytes)
    }
}

impl Arbitrary for StreamId {
    fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
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
