//! The [`GossipMessage`] protocol between Actyx nodes is encoded using [libipld].
//!
//! [libipld]: https://crates.io/crates/libipld
use crate::Block;
use actyx_sdk::{LamportTimestamp, Offset, StreamId, Timestamp};
use cbor_data::{
    codec::{CodecError, ReadCbor, WriteCbor},
    Encoder,
};
use libipld::Cid;
use std::collections::BTreeMap;

/// This is the union type for the pubsub protocol. Its wire format is extendable, as long as the
/// enum members' names are not reused.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum GossipMessage {
    RootUpdate(RootUpdate),
    RootMap(RootMap),
}

impl WriteCbor for GossipMessage {
    fn write_cbor<W: cbor_data::Writer>(&self, w: W) -> W::Output {
        match self {
            GossipMessage::RootUpdate(x) => w.encode_dict(|w| {
                w.with_key("RootUpdate", |w| x.write_cbor(w));
            }),
            GossipMessage::RootMap(x) => w.encode_dict(|w| {
                w.with_key("RootMap", |w| x.write_cbor(w));
            }),
        }
    }
}

impl ReadCbor for GossipMessage {
    fn fmt(f: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(f, "GossipMessage")
    }

    fn read_cbor(cbor: &cbor_data::Cbor) -> cbor_data::codec::Result<Self>
    where
        Self: Sized,
    {
        let d = cbor.try_dict()?;
        let d = d
            .iter()
            .filter_map(|(k, v)| k.decode().to_str().map(|k| (k, v)))
            .collect::<BTreeMap<_, _>>();
        if let Some(cbor) = d.get("RootUpdate") {
            return Ok(Self::RootUpdate(ReadCbor::read_cbor(cbor.as_ref())?));
        }
        if let Some(cbor) = d.get("RootMap") {
            return Ok(Self::RootMap(ReadCbor::read_cbor(cbor.as_ref())?));
        }
        Err(CodecError::str(format!(
            "no known variant found among {:?}",
            d.keys().collect::<Vec<_>>()
        )))
    }
}

/// This struct is used to publish an update to a single stream. The tree's block can either be
/// inlined (so called 'fast path') or omitted ('slow path'). If they are omitted, peers are
/// expected to resolve the blocks via bitswap.
///
/// **Wire format**: This struct is extendable, as it's encoded as an indefinite length map, and
/// older version will ignore unknown fields. They still need to be valid cbor though. The initial
/// version of Actyx v2 used a fixed size map, so this particular case needs to be special handled
/// while decoding updates from older nodes.
///
/// Up to including Actyx v2.3.1 the `offset` field was not present.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootUpdate {
    pub stream: StreamId,
    pub root: Cid,
    pub blocks: Vec<Block>,
    /// Lamport of the tree referenced by `root`
    pub lamport: LamportTimestamp,
    /// Message creation wallclock
    pub time: Timestamp,
    /// Offset of the tree referenced by `root`
    /// Optional for backwards compatibility
    pub offset: Option<Offset>,
}

impl WriteCbor for RootUpdate {
    fn write_cbor<W: cbor_data::Writer>(&self, w: W) -> W::Output {
        w.encode_dict(|w| {
            w.with_key("stream", |w| self.stream.write_cbor(w));
            w.with_key("root", |w| self.root.write_cbor(w));
            w.with_key("blocks", |w| self.blocks.write_cbor(w));
            w.with_key("lamport", |w| self.lamport.write_cbor(w));
            w.with_key("time", |w| self.time.write_cbor(w));
            w.with_key("offset", |w| self.offset.write_cbor(w));
        })
    }
}

impl ReadCbor for RootUpdate {
    fn fmt(f: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(f, "RootUpdate")
    }

    fn read_cbor(cbor: &cbor_data::Cbor) -> cbor_data::codec::Result<Self>
    where
        Self: Sized,
    {
        let d = cbor.try_dict()?;
        let d = d
            .iter()
            .filter_map(|(k, v)| k.decode().to_str().map(|k| (k, v)))
            .collect::<BTreeMap<_, _>>();
        Ok(Self {
            stream: ReadCbor::read_cbor(
                d.get("stream")
                    .ok_or_else(|| CodecError::str("missing field `stream`"))?
                    .as_ref(),
            )?,
            root: ReadCbor::read_cbor(
                d.get("root")
                    .ok_or_else(|| CodecError::str("missing field `root`"))?
                    .as_ref(),
            )?,
            blocks: ReadCbor::read_cbor(
                d.get("blocks")
                    .ok_or_else(|| CodecError::str("missing field `blocks`"))?
                    .as_ref(),
            )?,
            lamport: ReadCbor::read_cbor(
                d.get("lamport")
                    .ok_or_else(|| CodecError::str("missing field `lamport`"))?
                    .as_ref(),
            )?,
            time: ReadCbor::read_cbor(
                d.get("time")
                    .ok_or_else(|| CodecError::str("missing field `time`"))?
                    .as_ref(),
            )?,
            offset: if let Some(offset) = d.get("offset") {
                ReadCbor::read_cbor(offset.as_ref())?
            } else {
                Default::default()
            },
        })
    }
}

/// This struct represents a node's validated trees for a set of streams (incl. its own).
///
/// **Wire format**: This struct is extendable, as it's encoded as a infite length map, and older
/// version will ignore unknown fields. They still need to be valid cbor though. The initial
/// version of Actyx v2 used a fixed size map, so this particular case needs to be special handled
/// while decoding updates from older nodes.
///
/// Up to including Actyx v2.3.1 the `offsets` field was not present.
#[derive(Debug, Eq, PartialEq, Default, Clone)]
pub struct RootMap {
    pub entries: BTreeMap<StreamId, Cid>,
    /// Offset and lamport timestamp of the trees referenced in the `entries` map.
    /// Could be empty (backwards compatibilty!)
    pub offsets: Vec<(Offset, LamportTimestamp)>,
    /// Highest lamport timestamp known to the node at time of publishing the message
    pub lamport: LamportTimestamp,
    /// Message creation wallclock
    pub time: Timestamp,
}

impl WriteCbor for RootMap {
    fn write_cbor<W: cbor_data::Writer>(&self, w: W) -> W::Output {
        w.encode_dict(|w| {
            w.with_key("entries", |w| self.entries.write_cbor(w));
            w.with_key("lamport", |w| self.lamport.write_cbor(w));
            w.with_key("offsets", |w| self.offsets.write_cbor(w));
            w.with_key("time", |w| self.time.write_cbor(w));
        })
    }
}

impl ReadCbor for RootMap {
    fn fmt(f: &mut impl std::fmt::Write) -> std::fmt::Result {
        write!(f, "RootMap")
    }

    fn read_cbor(cbor: &cbor_data::Cbor) -> cbor_data::codec::Result<Self>
    where
        Self: Sized,
    {
        let d = cbor.try_dict()?;
        let d = d
            .iter()
            .filter_map(|(k, v)| k.decode().to_str().map(|k| (k, v)))
            .collect::<BTreeMap<_, _>>();
        Ok(Self {
            entries: ReadCbor::read_cbor(
                d.get("entries")
                    .ok_or_else(|| CodecError::str("missing field `entries`"))?
                    .as_ref(),
            )?,
            offsets: if let Some(offsets) = d.get("offsets") {
                ReadCbor::read_cbor(offsets.as_ref())?
            } else {
                Default::default()
            },
            lamport: ReadCbor::read_cbor(
                d.get("lamport")
                    .ok_or_else(|| CodecError::str("missing field `lamport`"))?
                    .as_ref(),
            )?,
            time: ReadCbor::read_cbor(
                d.get("time")
                    .ok_or_else(|| CodecError::str("missing field `time`"))?
                    .as_ref(),
            )?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyx_sdk::NodeId;
    use cbor_data::{Cbor, CborBuilder};
    use libipld::multihash::{Code, MultihashDigest};
    use quickcheck::Arbitrary;
    use quickcheck_macros::quickcheck;

    impl Arbitrary for GossipMessage {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            if bool::arbitrary(g) {
                GossipMessage::RootMap(RootMap::arbitrary(g))
            } else {
                GossipMessage::RootUpdate(RootUpdate::arbitrary(g))
            }
        }
    }

    impl Arbitrary for RootMap {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut offsets = vec![];
            let len = g.size();
            let entries = (0..len)
                .into_iter()
                .map(|_| {
                    offsets.push((Arbitrary::arbitrary(g), Arbitrary::arbitrary(g)));
                    let cid = Cid::new_v1(0x00, Code::Sha2_256.digest(&Vec::<u8>::arbitrary(g)[..]));
                    (Arbitrary::arbitrary(g), cid)
                })
                .collect();
            Self {
                entries,
                offsets,
                lamport: Arbitrary::arbitrary(g),
                time: Arbitrary::arbitrary(g),
            }
        }
    }

    impl Arbitrary for RootUpdate {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let root = Cid::new_v1(0x00, Code::Sha2_256.digest(&Vec::<u8>::arbitrary(g)[..]));

            let len = g.size();
            let blocks = (0..len)
                .into_iter()
                .map(|_| {
                    let data = Vec::<u8>::arbitrary(g);
                    let cid = Cid::new_v1(0x00, Code::Sha2_256.digest(&data[..]));
                    crate::Block::new_unchecked(cid, data)
                })
                .collect();
            Self {
                stream: Arbitrary::arbitrary(g),
                root,
                blocks,
                lamport: Arbitrary::arbitrary(g),
                time: Arbitrary::arbitrary(g),
                offset: Arbitrary::arbitrary(g),
            }
        }
    }

    #[quickcheck]
    fn roundtrip_new(message: GossipMessage) -> bool {
        let bytes = message.write_cbor(CborBuilder::default());
        let decoded: GossipMessage = ReadCbor::read_cbor(bytes.as_ref()).unwrap();
        decoded == message
    }

    #[test]
    fn test_decode_root_update() {
        #[rustfmt::skip]
        let cbor = [
            0xa1, // map(1)
                0x6a, // string(10)
                    b'R', b'o', b'o', b't', b'U', b'p', b'd', b'a', b't', b'e',
                0xbf, // map(infinitze size)
                    0x66, // string(6)
                        b'b', b'l', b'o', b'c', b'k', b's',
                    0x80, // array(0)
                    0x67, // string(7)
                        b'l', b'a', b'm', b'p', b'o', b'r', b't',
                    0x00, // unsigned(0)
                    0x64, // string(4)
                        b'r', b'o', b'o', b't',
                    0xd8, 0x2a, // tag(42)
                    0x58, 0x25, // bytes(37)
                        0x00, 0x01, 0x00, 0x12,
                        0x20, 0xE3, 0xB0, 0xC4,
                        0x42, 0x98, 0xFC, 0x1C,
                        0x14, 0x9A, 0xFB, 0xF4,
                        0xC8, 0x99, 0x6F, 0xB9,
                        0x24, 0x27, 0xAE, 0x41,
                        0xE4, 0x64, 0x9B, 0x93,
                        0x4C, 0xA4, 0x95, 0x99,
                        0x1B, 0x78, 0x52, 0xB8, 0x55,
                    0x66, // string(6)
                        b's', b't', b'r', b'e', b'a', b'm',
                    0x82, // array(2)
                        0x58, 0x20, // bytes(32)
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                        0x18, 0x2a, // unsigned(42)
                    0x64, // string(4)
                        b't', b'i', b'm', b'e',
                    0x00, // unsigned(0)
                    0x66, // string(6)
                        b'o', b'f', b'f', b's', b'e', b't',
                    0xF6, // null
                0xff // break map
        ];
        #[rustfmt::skip]
        let cbor_out = [
            0xa1, // map(1)
                0x6a, // string(10)
                    b'R', b'o', b'o', b't', b'U', b'p', b'd', b'a', b't', b'e',
                0xa6, // map(length 6)
                    0x66, // string(6)
                        b's', b't', b'r', b'e', b'a', b'm',
                    0x82, // array(2)
                        0x58, 0x20, // bytes(32)
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                        0x18, 0x2a, // unsigned(42)
                    0x64, // string(4)
                        b'r', b'o', b'o', b't',
                    0xd8, 0x2a, // tag(42)
                    0x58, 0x25, // bytes(37)
                        0x00, 0x01, 0x00, 0x12,
                        0x20, 0xE3, 0xB0, 0xC4,
                        0x42, 0x98, 0xFC, 0x1C,
                        0x14, 0x9A, 0xFB, 0xF4,
                        0xC8, 0x99, 0x6F, 0xB9,
                        0x24, 0x27, 0xAE, 0x41,
                        0xE4, 0x64, 0x9B, 0x93,
                        0x4C, 0xA4, 0x95, 0x99,
                        0x1B, 0x78, 0x52, 0xB8, 0x55,
                    0x66, // string(6)
                        b'b', b'l', b'o', b'c', b'k', b's',
                    0x80, // array(0)
                    0x67, // string(7)
                        b'l', b'a', b'm', b'p', b'o', b'r', b't',
                    0x00, // unsigned(0)
                    0x64, // string(4)
                        b't', b'i', b'm', b'e',
                    0x00, // unsigned(0)
                    0x66, // string(6)
                        b'o', b'f', b'f', b's', b'e', b't',
                    0xF6, // null
        ];
        let root_update = GossipMessage::RootUpdate(RootUpdate {
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(42.into()),
            root: Cid::new_v1(0x00, Code::Sha2_256.digest(&[])),
            blocks: Default::default(),
            lamport: Default::default(),
            time: Default::default(),
            offset: None,
        });
        let msg = root_update.write_cbor(CborBuilder::default());
        assert_eq!(
            msg.as_slice(),
            cbor_out,
            "\nleft:  {:X?}\nright: {:X?}",
            msg.as_slice(),
            cbor_out
        );
        let root_update2 = GossipMessage::read_cbor(&*msg).unwrap();
        assert_eq!(root_update, root_update2);
        let root_update3 = GossipMessage::read_cbor(Cbor::checked(&cbor[..]).unwrap()).unwrap();
        assert_eq!(root_update, root_update3);
    }

    #[test]
    fn test_decode_root_map() {
        #[rustfmt::skip]
        let cbor = [
            0xbf, // map(infinite length)
                0x67, // string(7)
                    b'e', b'n', b't', b'r', b'i', b'e', b's',
                0xa0, // map(0)
                0x67, // string(7)
                    b'l', b'a', b'm', b'p', b'o', b'r', b't',
                0x00, // unsigned(0)
                0x67, // string(7)
                    b'o', b'f', b'f', b's', b'e', b't', b's',
                0x80, // array(0)
                0x64, // string(4)
                    b't', b'i', b'm', b'e',
                0x00, // unsigned(0)
            0xff // break
        ];
        #[rustfmt::skip]
        let cbor_out = [
            0xa4, // map(length 4)
                0x67, // string(7)
                    b'e', b'n', b't', b'r', b'i', b'e', b's',
                0xa0, // map(0)
                0x67, // string(7)
                    b'l', b'a', b'm', b'p', b'o', b'r', b't',
                0x00, // unsigned(0)
                0x67, // string(7)
                    b'o', b'f', b'f', b's', b'e', b't', b's',
                0x80, // array(0)
                0x64, // string(4)
                    b't', b'i', b'm', b'e',
                0x00, // unsigned(0)
        ];
        let root_map = RootMap::default();
        let msg = root_map.write_cbor(CborBuilder::default());
        assert_eq!(msg.as_slice(), cbor_out);
        let root_map2 = RootMap::read_cbor(&*msg).unwrap();
        assert_eq!(root_map, root_map2);
        let root_map3 = RootMap::read_cbor(Cbor::checked(&cbor[..]).unwrap()).unwrap();
        assert_eq!(root_map3, root_map);
    }
}
