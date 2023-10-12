// Used by trees::arb
// Can't really be moved to `trees` since Rust disallows
// implementing foreign traits on foreign types

use std::collections::BTreeMap;

use crate::{
    language::SortKey,
    offset::{Offset, OffsetMap, OffsetOrMin},
    scalars::{NodeId, StreamId, StreamNr},
    service::EventMeta,
    EventKey, LamportTimestamp, Metadata, Tag, TagSet, Timestamp,
};
use quickcheck::{Arbitrary, Gen};

impl Arbitrary for NodeId {
    fn arbitrary(g: &mut Gen) -> Self {
        let x = u128::arbitrary(g);
        let mut bytes = [0u8; 32];
        bytes[0..16].copy_from_slice(&x.to_be_bytes());
        bytes[16..32].copy_from_slice(&x.to_le_bytes());
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

impl Arbitrary for Timestamp {
    fn arbitrary(g: &mut Gen) -> Self {
        Timestamp::new(u64::arbitrary(g) & ((2 << 53) - 1))
    }
}

impl Arbitrary for LamportTimestamp {
    fn arbitrary(g: &mut Gen) -> Self {
        LamportTimestamp::new(u64::arbitrary(g))
    }
}

impl Arbitrary for SortKey {
    fn arbitrary(g: &mut Gen) -> Self {
        Self {
            lamport: Arbitrary::arbitrary(g),
            stream: Arbitrary::arbitrary(g),
        }
    }
}

impl Arbitrary for EventKey {
    fn arbitrary(g: &mut Gen) -> Self {
        Self {
            lamport: Arbitrary::arbitrary(g),
            stream: Arbitrary::arbitrary(g),
            offset: Arbitrary::arbitrary(g),
        }
    }
}

impl Arbitrary for EventMeta {
    fn arbitrary(g: &mut Gen) -> Self {
        enum Kind {
            S,
            E,
            R,
        }
        match g.choose(&[Kind::S, Kind::E, Kind::R]).unwrap() {
            Kind::S => EventMeta::Synthetic,
            Kind::E => EventMeta::Event {
                key: Arbitrary::arbitrary(g),
                meta: Arbitrary::arbitrary(g),
            },
            Kind::R => EventMeta::Range {
                from_key: Arbitrary::arbitrary(g),
                to_key: Arbitrary::arbitrary(g),
                from_time: Arbitrary::arbitrary(g),
                to_time: Arbitrary::arbitrary(g),
            },
        }
    }
}

impl Arbitrary for Metadata {
    fn arbitrary(g: &mut Gen) -> Self {
        Self {
            timestamp: Arbitrary::arbitrary(g),
            tags: Arbitrary::arbitrary(g),
            app_id: Arbitrary::arbitrary(g),
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

impl Arbitrary for Tag {
    fn arbitrary(g: &mut Gen) -> Self {
        let size = g.size().max(1);
        let inner: String = (0..size).map(|_| char::arbitrary(g)).collect();
        inner.parse().expect("non empty string")
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let str: String = self.to_string();
        Box::new(
            str.shrink()
                .filter(|x| !x.is_empty())
                .map(|x| x.parse().expect("non empty")),
        )
    }
}

impl Arbitrary for TagSet {
    fn arbitrary(g: &mut Gen) -> Self {
        let inner: Vec<Tag> = Arbitrary::arbitrary(g);
        inner.into()
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let tags: Vec<Tag> = self.iter().collect();
        Box::new(tags.shrink().map(Into::into))
    }
}
