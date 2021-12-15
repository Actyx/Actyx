//! The [`GossipMessage`] protocol between Actyx nodes is encoded using [libipld].
//!
//! [libipld]: https://crates.io/crates/libipld
use crate::Block;
use actyx_sdk::{LamportTimestamp, Offset, StreamId, Timestamp};
use anyhow::Result;
use libipld::{
    cbor::DagCborCodec,
    codec::{Decode, Encode},
    Cid, DagCbor,
};
use std::collections::BTreeMap;

/// This is the union type for the pubsub protocol. Its wire format is extendable, as long as the
/// enum members' names are not reused.
#[derive(Debug, Eq, PartialEq, DagCbor, Clone)]
#[ipld(repr = "keyed")]
pub enum GossipMessage {
    #[ipld(repr = "value")]
    RootUpdate(RootUpdate),
    #[ipld(repr = "value")]
    RootMap(RootMap),
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

impl Encode<DagCborCodec> for RootUpdate {
    fn encode<W: std::io::Write>(&self, c: libipld::cbor::DagCborCodec, w: &mut W) -> libipld::Result<()> {
        let RootUpdate {
            ref stream,
            ref root,
            ref blocks,
            ref lamport,
            ref time,
            ref offset,
        } = self;

        let blocks: Vec<(Cid, Vec<u8>)> = blocks
            .iter()
            .map(|block| (*block.cid(), block.data().to_vec()))
            .collect();

        // TODO: We might want to use `cbor_data` instead of this magic number fiddling at some
        // point.

        w.write_all(&[0xBF])?; // IL Map
        Encode::encode("blocks", c, w)?;
        Encode::encode(&blocks, c, w)?;
        Encode::encode("lamport", c, w)?;
        Encode::encode(lamport, c, w)?;
        Encode::encode("root", c, w)?;
        Encode::encode(root, c, w)?;
        Encode::encode("stream", c, w)?;
        Encode::encode(stream, c, w)?;
        Encode::encode("time", c, w)?;
        Encode::encode(time, c, w)?;
        Encode::encode("offset", c, w)?;
        Encode::encode(offset, c, w)?;
        w.write_all(&[0xFF])?; // break

        Ok(())
    }
}

impl Decode<DagCborCodec> for RootUpdate {
    fn decode<R: std::io::Read + std::io::Seek>(c: libipld::cbor::DagCborCodec, r: &mut R) -> libipld::Result<Self> {
        use libipld::cbor::decode::read_u8;
        use libipld::cbor::error::{MissingKey, UnexpectedCode};
        use std::io::SeekFrom;
        let major = read_u8(r)?;
        match major {
            0xa5 => {
                // Compatibility with Actyx <= 2.3.1
                let mut blocks: Option<Vec<(Cid, Vec<u8>)>> = None;
                let mut lamport = None;
                let mut root = None;
                let mut stream = None;
                let mut time = None;
                for _ in 0..5 {
                    let key: String = Decode::decode(c, r)?;
                    match key.as_str() {
                        "blocks" => {
                            blocks = Some(Decode::decode(c, r)?);
                        }
                        "lamport" => {
                            lamport = Some(Decode::decode(c, r)?);
                        }
                        "root" => {
                            root = Some(Decode::decode(c, r)?);
                        }
                        "stream" => {
                            stream = Some(Decode::decode(c, r)?);
                        }
                        "time" => {
                            time = Some(Decode::decode(c, r)?);
                        }

                        _ => {
                            libipld::Ipld::decode(c, r)?;
                        }
                    }
                }
                let blocks = blocks
                    .ok_or_else(|| MissingKey::new::<Self>("blocks"))?
                    .into_iter()
                    .map(|(cid, data)| Block::new(cid, data.to_vec()))
                    .collect::<Result<Vec<Block>>>()?;

                let lamport = lamport.ok_or_else(|| MissingKey::new::<Self>("lamport"))?;
                let root = root.ok_or_else(|| MissingKey::new::<Self>("root"))?;
                let stream = stream.ok_or_else(|| MissingKey::new::<Self>("stream"))?;
                let time = time.ok_or_else(|| MissingKey::new::<Self>("time"))?;
                Ok(RootUpdate {
                    stream,
                    root,
                    blocks,
                    lamport,
                    time,
                    offset: None,
                })
            }
            0xbf => {
                // current version
                let mut blocks: Option<Vec<(Cid, Vec<u8>)>> = None;
                let mut lamport = None;
                let mut root = None;
                let mut stream = None;
                let mut time = None;
                let mut offset = None;
                while read_u8(r)? != 0xff {
                    r.seek(SeekFrom::Current(-1))?;
                    let key = String::decode(c, r)?;
                    match key.as_str() {
                        "blocks" => {
                            blocks = Some(Decode::decode(c, r)?);
                        }
                        "lamport" => {
                            lamport = Some(Decode::decode(c, r)?);
                        }
                        "root" => {
                            root = Some(Decode::decode(c, r)?);
                        }
                        "stream" => {
                            stream = Some(Decode::decode(c, r)?);
                        }
                        "time" => {
                            time = Some(Decode::decode(c, r)?);
                        }
                        "offset" => {
                            offset = Some(Decode::decode(c, r)?);
                        }
                        _ => {
                            libipld::Ipld::decode(c, r)?;
                        }
                    }
                }
                let blocks = blocks
                    .ok_or_else(|| MissingKey::new::<Self>("blocks"))?
                    .into_iter()
                    .map(|(cid, data)| Block::new(cid, data.to_vec()))
                    .collect::<Result<Vec<Block>>>()?;
                let lamport = lamport.ok_or_else(|| MissingKey::new::<Self>("lamport"))?;
                let root = root.ok_or_else(|| MissingKey::new::<Self>("root"))?;
                let stream = stream.ok_or_else(|| MissingKey::new::<Self>("stream"))?;
                let time = time.ok_or_else(|| MissingKey::new::<Self>("time"))?;
                let offset = offset.ok_or_else(|| MissingKey::new::<Self>("offset"))?;
                Ok(RootUpdate {
                    stream,
                    root,
                    blocks,
                    lamport,
                    time,
                    offset,
                })
            }
            _ => Err(UnexpectedCode::new::<Self>(major).into()),
        }
    }
}

/// This struct represents a node's validated trees for a set of streams (incl. its own).
///
/// **Wire format**: This struct is extendable, as it's encoded as a infite length map, and older
/// version will ignore unknown fields. They still need to be valid cbor though. The initial
/// version of Actyx v2 used a fixed size map, so this particular case needs to be special handled
/// while decoding updates from older nodes.
///
/// Up to including Actyx v2.3.1 the `entries` field was not present.
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

impl libipld::codec::Encode<libipld::cbor::DagCborCodec> for RootMap {
    fn encode<W: std::io::Write>(&self, c: libipld::cbor::DagCborCodec, w: &mut W) -> libipld::Result<()> {
        let RootMap {
            ref entries,
            ref offsets,
            ref lamport,
            ref time,
        } = self;
        w.write_all(&[0xbf])?;
        Encode::encode("entries", c, w)?;
        Encode::encode(entries, c, w)?;
        Encode::encode("lamport", c, w)?;
        Encode::encode(lamport, c, w)?;
        Encode::encode("offsets", c, w)?;
        Encode::encode(offsets, c, w)?;
        Encode::encode("time", c, w)?;
        Encode::encode(time, c, w)?;
        w.write_all(&[0xff])?;

        Ok(())
    }
}
impl libipld::codec::Decode<libipld::cbor::DagCborCodec> for RootMap {
    fn decode<R: std::io::Read + std::io::Seek>(c: libipld::cbor::DagCborCodec, r: &mut R) -> libipld::Result<Self> {
        use libipld::cbor::decode::read_u8;
        use libipld::cbor::error::{MissingKey, UnexpectedCode};
        use std::io::SeekFrom;
        let major = read_u8(r)?;
        match major {
            0xa3 => {
                // Compatibility with Actyx <= 2.3.1
                let mut entries = None;
                let mut lamport = None;
                let mut time = None;
                for _ in 0..3 {
                    let key: String = Decode::decode(c, r)?;
                    match key.as_str() {
                        "entries" => {
                            entries = Some(Decode::decode(c, r)?);
                        }
                        "lamport" => {
                            lamport = Some(Decode::decode(c, r)?);
                        }
                        "time" => {
                            time = Some(Decode::decode(c, r)?);
                        }
                        _ => {
                            libipld::Ipld::decode(c, r)?;
                        }
                    }
                }
                let entries = entries.ok_or_else(|| MissingKey::new::<Self>("entries"))?;
                let lamport = lamport.ok_or_else(|| MissingKey::new::<Self>("lamport"))?;
                let time = time.ok_or_else(|| MissingKey::new::<Self>("time"))?;
                Ok(RootMap {
                    entries,
                    offsets: vec![],
                    lamport,
                    time,
                })
            }
            0xbf => {
                // new version
                let mut entries = None;
                let mut lamport = None;
                let mut offsets = None;
                let mut time = None;
                while read_u8(r)? != 0xff {
                    r.seek(SeekFrom::Current(-1))?;
                    let key = String::decode(c, r)?;
                    match key.as_str() {
                        "entries" => {
                            entries = Some(Decode::decode(c, r)?);
                        }
                        "lamport" => {
                            lamport = Some(Decode::decode(c, r)?);
                        }
                        "offsets" => {
                            offsets = Some(Decode::decode(c, r)?);
                        }
                        "time" => {
                            time = Some(Decode::decode(c, r)?);
                        }
                        _ => {
                            libipld::Ipld::decode(c, r)?;
                        }
                    }
                }
                let entries = entries.ok_or_else(|| MissingKey::new::<Self>("entries"))?;
                let lamport = lamport.ok_or_else(|| MissingKey::new::<Self>("lamport"))?;
                let offsets = offsets.ok_or_else(|| MissingKey::new::<Self>("offsets"))?;
                let time = time.ok_or_else(|| MissingKey::new::<Self>("time"))?;
                Ok(RootMap {
                    entries,
                    offsets,
                    lamport,
                    time,
                })
            }
            _ => Err(UnexpectedCode::new::<Self>(major).into()),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use actyx_sdk::NodeId;
    use libipld::{
        codec::Codec,
        multihash::{Code, MultihashDigest},
    };
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
        let bytes = DagCborCodec.encode(&message).unwrap();
        let decoded: GossipMessage = DagCborCodec.decode(&bytes).unwrap();
        decoded == message
    }

    #[quickcheck]
    fn roundtrip_old(message: old::GossipMessage) -> bool {
        let bytes = DagCborCodec.encode(&message).unwrap();
        let decoded: old::GossipMessage = DagCborCodec.decode(&bytes).unwrap();
        decoded == message
    }

    #[quickcheck]
    fn roundtrip_new_to_old(message: GossipMessage) -> bool {
        let bytes = DagCborCodec.encode(&message).unwrap();
        let decoded: old::GossipMessage = DagCborCodec.decode(&bytes).unwrap();
        match (decoded, message) {
            (old::GossipMessage::RootUpdate(x), GossipMessage::RootUpdate(y)) => {
                x.stream == y.stream && x.root == y.root && x.blocks == y.blocks && x.lamport == y.lamport
            }
            (old::GossipMessage::RootMap(x), GossipMessage::RootMap(y)) => {
                x.entries == y.entries && x.lamport == y.lamport && x.time == y.time
            }
            _ => false,
        }
    }

    #[quickcheck]
    fn roundtrip_old_to_new(message: old::GossipMessage) -> bool {
        let bytes = DagCborCodec.encode(&message).unwrap();
        let decoded: GossipMessage = DagCborCodec.decode(&bytes).unwrap();
        match (decoded, message) {
            (GossipMessage::RootUpdate(x), old::GossipMessage::RootUpdate(y)) => {
                x.stream == y.stream
                    && x.root == y.root
                    && x.blocks == y.blocks
                    && x.lamport == y.lamport
                    && x.offset.is_none()
            }
            (GossipMessage::RootMap(x), old::GossipMessage::RootMap(y)) => {
                x.entries == y.entries && x.lamport == y.lamport && x.time == y.time && x.offsets.is_empty()
            }
            _ => false,
        }
    }

    #[test]
    fn test_decode_root_update_old() {
        #[rustfmt::skip]
        let cbor = [
            0xa5, // map(5)
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
        ];
        let root_update = old::RootUpdate {
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(42.into()),
            root: Cid::new_v1(0x00, Code::Sha2_256.digest(&[])),
            blocks: Default::default(),
            lamport: Default::default(),
            time: Default::default(),
        };
        let msg = DagCborCodec.encode(&root_update).unwrap();
        assert_eq!(msg, cbor);
        let root_update2 = DagCborCodec.decode(&msg).unwrap();
        assert_eq!(root_update, root_update2);
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
        let root_update = GossipMessage::RootUpdate(RootUpdate {
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(42.into()),
            root: Cid::new_v1(0x00, Code::Sha2_256.digest(&[])),
            blocks: Default::default(),
            lamport: Default::default(),
            time: Default::default(),
            offset: None,
        });
        let msg = DagCborCodec.encode(&root_update).unwrap();
        assert_eq!(msg, cbor, "left: {:X?}\nright: {:X?}", msg, cbor);
        let root_update2 = DagCborCodec.decode(&msg).unwrap();
        assert_eq!(root_update, root_update2);
        let root_update3: GossipMessage = DagCborCodec.decode(&cbor[..]).unwrap();
        assert_eq!(root_update, root_update3);
    }

    #[test]
    fn test_root_update_backwards_compatibility() {
        // decode old format
        let old = old::RootUpdate {
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(42.into()),
            root: Cid::new_v1(0x00, Code::Sha2_256.digest(&[])),
            blocks: Default::default(),
            lamport: Default::default(),
            time: Default::default(),
        };
        let expected_new = RootUpdate {
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(42.into()),
            root: Cid::new_v1(0x00, Code::Sha2_256.digest(&[])),
            blocks: Default::default(),
            lamport: Default::default(),
            time: Default::default(),
            offset: None,
        };
        let bytes = DagCborCodec.encode(&old).unwrap();
        let decoded: RootUpdate = DagCborCodec.decode(&bytes[..]).unwrap();
        assert_eq!(expected_new, decoded);
        // decode new format

        let new = RootUpdate {
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(42.into()),
            root: Cid::new_v1(0x00, Code::Sha2_256.digest(&[])),
            blocks: Default::default(),
            lamport: Default::default(),
            time: Default::default(),
            offset: None,
        };
        let expected_old = old::RootUpdate {
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(42.into()),
            root: Cid::new_v1(0x00, Code::Sha2_256.digest(&[])),
            blocks: Default::default(),
            lamport: Default::default(),
            time: Default::default(),
        };
        let bytes = DagCborCodec.encode(&new).unwrap();
        let decoded: old::RootUpdate = DagCborCodec.decode(&bytes[..]).unwrap();
        assert_eq!(expected_old, decoded);
    }

    #[test]
    fn test_decode_root_map_old() {
        #[rustfmt::skip]
        let cbor = [
            0xa3, // map(3)
                0x67, // string(7)
                    b'e', b'n', b't', b'r', b'i', b'e', b's',
                0xa0, // map(0)
                0x67, // string(7)
                    b'l', b'a', b'm', b'p', b'o', b'r', b't',
                0x00, // unsigned(0)
                0x64, // string(4)
                    b't', b'i', b'm', b'e',
                0x00, // unsigned(0)
        ];
        let root_map = old::RootMap::default();
        let msg = DagCborCodec.encode(&root_map).unwrap();
        assert_eq!(msg, cbor);
        let root_map2: old::RootMap = DagCborCodec.decode(&msg).unwrap();
        assert_eq!(root_map, root_map2);
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
        let root_map = RootMap::default();
        let msg = DagCborCodec.encode(&root_map).unwrap();
        assert_eq!(msg, cbor);
        let root_map2 = DagCborCodec.decode(&msg).unwrap();
        assert_eq!(root_map, root_map2);
    }

    #[test]
    fn test_root_map_backwards_compatibility() {
        let old = old::RootMap::default();
        let new = RootMap::default();
        // read old
        let bytes = DagCborCodec.encode(&old).unwrap();
        let decoded: RootMap = DagCborCodec.decode(&bytes[..]).unwrap();
        assert_eq!(decoded, new);

        // read new
        let bytes = DagCborCodec.encode(&new).unwrap();
        let decoded: old::RootMap = DagCborCodec.decode(&bytes[..]).unwrap();
        assert_eq!(decoded, old);
    }

    #[allow(clippy::all)]
    mod old {
        //! Inlined encoding and decoding functions for the Gossip Protocol for Actyx <= 2.3.1 to
        //! ensure backwards compatibility.
        use actyx_sdk::{LamportTimestamp, StreamId, Timestamp};
        use anyhow::Result;
        use ipfs_embed::Cid;
        use libipld::{
            cbor::{encode::write_u64, DagCborCodec},
            codec::{Decode, Encode},
        };
        use quickcheck::Arbitrary;
        use std::{collections::BTreeMap, convert::TryFrom};

        use crate::Block;

        #[derive(Debug, Eq, PartialEq, Clone)]
        pub(crate) enum GossipMessage {
            RootUpdate(RootUpdate),
            RootMap(RootMap),
        }
        impl libipld::codec::Encode<libipld::cbor::DagCborCodec> for GossipMessage {
            fn encode<W: std::io::Write>(&self, c: libipld::cbor::DagCborCodec, w: &mut W) -> libipld::Result<()> {
                match *self {
                    GossipMessage::RootUpdate(ref __binding_0) => {
                        write_u64(w, 5, 1)?;
                        Encode::encode("RootUpdate", c, w)?;
                        Encode::encode(__binding_0, c, w)?;
                    }
                    GossipMessage::RootMap(ref __binding_0) => {
                        write_u64(w, 5, 1)?;
                        Encode::encode("RootMap", c, w)?;
                        Encode::encode(__binding_0, c, w)?;
                    }
                }
                Ok(())
            }
        }
        impl libipld::codec::Decode<libipld::cbor::DagCborCodec> for GossipMessage {
            fn decode<R: std::io::Read + std::io::Seek>(
                c: libipld::cbor::DagCborCodec,
                r: &mut R,
            ) -> libipld::Result<Self> {
                use libipld::cbor::decode::read_u8;
                use libipld::cbor::error::{UnexpectedCode, UnexpectedKey};
                let major = read_u8(r)?;
                if major != 0xa1 {
                    return Err(UnexpectedCode::new::<Self>(major).into());
                }
                let key: String = Decode::decode(c, r)?;
                if key.as_str() == "RootUpdate" {
                    let __binding_0 = Decode::decode(c, r)?;
                    return Ok(GossipMessage::RootUpdate(__binding_0));
                };
                if key.as_str() == "RootMap" {
                    let __binding_0 = Decode::decode(c, r)?;
                    return Ok(GossipMessage::RootMap(__binding_0));
                };
                Err(UnexpectedKey::new::<Self>(key).into())
            }
        }
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub(crate) struct RootUpdate {
            pub(crate) stream: StreamId,
            pub(crate) root: Cid,
            pub(crate) blocks: Vec<Block>,
            pub(crate) lamport: LamportTimestamp,
            pub(crate) time: Timestamp,
        }

        impl Encode<DagCborCodec> for RootUpdate {
            fn encode<W: std::io::Write>(&self, c: DagCborCodec, w: &mut W) -> Result<()> {
                RootUpdateIo::from(self).encode(c, w)
            }
        }

        impl Decode<DagCborCodec> for RootUpdate {
            fn decode<R: std::io::Read + std::io::Seek>(c: DagCborCodec, r: &mut R) -> Result<Self> {
                let tmp = RootUpdateIo::decode::<R>(c, r)?;
                RootUpdate::try_from(tmp)
            }
        }

        struct RootUpdateIo {
            stream: StreamId,
            root: Cid,
            blocks: Vec<(Cid, Vec<u8>)>,
            lamport: LamportTimestamp,
            time: Timestamp,
        }
        impl libipld::codec::Encode<libipld::cbor::DagCborCodec> for RootUpdateIo {
            fn encode<W: std::io::Write>(&self, c: libipld::cbor::DagCborCodec, w: &mut W) -> libipld::Result<()> {
                match *self {
                    RootUpdateIo {
                        stream: ref __binding_0,
                        root: ref __binding_1,
                        blocks: ref __binding_2,
                        lamport: ref __binding_3,
                        time: ref __binding_4,
                    } => {
                        let len = 5u64;
                        write_u64(w, 5, len)?;
                        Encode::encode("blocks", c, w)?;
                        Encode::encode(__binding_2, c, w)?;
                        Encode::encode("lamport", c, w)?;
                        Encode::encode(__binding_3, c, w)?;
                        Encode::encode("root", c, w)?;
                        Encode::encode(__binding_1, c, w)?;
                        Encode::encode("stream", c, w)?;
                        Encode::encode(__binding_0, c, w)?;
                        Encode::encode("time", c, w)?;
                        Encode::encode(__binding_4, c, w)?;
                    }
                }
                Ok(())
            }
        }
        impl libipld::codec::Decode<libipld::cbor::DagCborCodec> for RootUpdateIo {
            fn decode<R: std::io::Read + std::io::Seek>(
                c: libipld::cbor::DagCborCodec,
                r: &mut R,
            ) -> libipld::Result<Self> {
                use libipld::cbor::decode::{read_len, read_u8};
                use libipld::cbor::error::{LengthOutOfRange, MissingKey, UnexpectedCode};
                use std::io::SeekFrom;
                let major = read_u8(r)?;
                match major {
                    0xa0..=0xbb => {
                        let len = read_len(r, major - 0xa0)?;
                        if len > 5usize {
                            return Err(LengthOutOfRange::new::<Self>().into());
                        }
                        let mut __binding_2 = None;
                        let mut __binding_3 = None;
                        let mut __binding_1 = None;
                        let mut __binding_0 = None;
                        let mut __binding_4 = None;
                        for _ in 0..len {
                            let key: String = Decode::decode(c, r)?;
                            match key.as_str() {
                                "blocks" => {
                                    __binding_2 = Some(Decode::decode(c, r)?);
                                }
                                "lamport" => {
                                    __binding_3 = Some(Decode::decode(c, r)?);
                                }
                                "root" => {
                                    __binding_1 = Some(Decode::decode(c, r)?);
                                }
                                "stream" => {
                                    __binding_0 = Some(Decode::decode(c, r)?);
                                }
                                "time" => {
                                    __binding_4 = Some(Decode::decode(c, r)?);
                                }
                                _ => {
                                    libipld::Ipld::decode(c, r)?;
                                }
                            }
                        }
                        let __binding_2 = __binding_2.ok_or(MissingKey::new::<Self>("blocks"))?;
                        let __binding_3 = __binding_3.ok_or(MissingKey::new::<Self>("lamport"))?;
                        let __binding_1 = __binding_1.ok_or(MissingKey::new::<Self>("root"))?;
                        let __binding_0 = __binding_0.ok_or(MissingKey::new::<Self>("stream"))?;
                        let __binding_4 = __binding_4.ok_or(MissingKey::new::<Self>("time"))?;
                        return Ok(RootUpdateIo {
                            stream: __binding_0,
                            root: __binding_1,
                            blocks: __binding_2,
                            lamport: __binding_3,
                            time: __binding_4,
                        });
                    }
                    0xbf => {
                        let mut __binding_2 = None;
                        let mut __binding_3 = None;
                        let mut __binding_1 = None;
                        let mut __binding_0 = None;
                        let mut __binding_4 = None;
                        loop {
                            let major = read_u8(r)?;
                            if major == 0xff {
                                break;
                            }
                            r.seek(SeekFrom::Current(-1))?;
                            let key = String::decode(c, r)?;
                            match key.as_str() {
                                "blocks" => {
                                    __binding_2 = Some(Decode::decode(c, r)?);
                                }
                                "lamport" => {
                                    __binding_3 = Some(Decode::decode(c, r)?);
                                }
                                "root" => {
                                    __binding_1 = Some(Decode::decode(c, r)?);
                                }
                                "stream" => {
                                    __binding_0 = Some(Decode::decode(c, r)?);
                                }
                                "time" => {
                                    __binding_4 = Some(Decode::decode(c, r)?);
                                }
                                _ => {
                                    libipld::Ipld::decode(c, r)?;
                                }
                            }
                        }
                        let __binding_2 = __binding_2.ok_or(MissingKey::new::<Self>("blocks"))?;
                        let __binding_3 = __binding_3.ok_or(MissingKey::new::<Self>("lamport"))?;
                        let __binding_1 = __binding_1.ok_or(MissingKey::new::<Self>("root"))?;
                        let __binding_0 = __binding_0.ok_or(MissingKey::new::<Self>("stream"))?;
                        let __binding_4 = __binding_4.ok_or(MissingKey::new::<Self>("time"))?;
                        return Ok(RootUpdateIo {
                            stream: __binding_0,
                            root: __binding_1,
                            blocks: __binding_2,
                            lamport: __binding_3,
                            time: __binding_4,
                        });
                    }
                    _ => {
                        return Err(UnexpectedCode::new::<Self>(major).into());
                    }
                }
            }
        }

        impl From<&RootUpdate> for RootUpdateIo {
            fn from(value: &RootUpdate) -> Self {
                Self {
                    stream: value.stream,
                    root: value.root,
                    blocks: value
                        .blocks
                        .iter()
                        .map(|block| (*block.cid(), block.data().to_vec()))
                        .collect(),
                    lamport: value.lamport,
                    time: value.time,
                }
            }
        }

        impl TryFrom<RootUpdateIo> for RootUpdate {
            type Error = anyhow::Error;

            fn try_from(value: RootUpdateIo) -> Result<Self, Self::Error> {
                let blocks = value
                    .blocks
                    .into_iter()
                    .map(|(cid, data)| Block::new(cid, data.to_vec()))
                    .collect::<Result<Vec<Block>>>()?;
                Ok(Self {
                    root: value.root,
                    stream: value.stream,
                    lamport: value.lamport,
                    time: value.time,
                    blocks,
                })
            }
        }

        #[derive(Debug, Eq, PartialEq, Default, Clone)]
        pub(crate) struct RootMap {
            pub(crate) entries: BTreeMap<StreamId, Cid>,
            pub(crate) lamport: LamportTimestamp,
            pub(crate) time: Timestamp,
        }
        impl libipld::codec::Encode<libipld::cbor::DagCborCodec> for RootMap {
            fn encode<W: std::io::Write>(&self, c: libipld::cbor::DagCborCodec, w: &mut W) -> libipld::Result<()> {
                match *self {
                    RootMap {
                        entries: ref __binding_0,
                        lamport: ref __binding_1,
                        time: ref __binding_2,
                    } => {
                        let len = 3u64;
                        write_u64(w, 5, len)?;
                        Encode::encode("entries", c, w)?;
                        Encode::encode(__binding_0, c, w)?;
                        Encode::encode("lamport", c, w)?;
                        Encode::encode(__binding_1, c, w)?;
                        Encode::encode("time", c, w)?;
                        Encode::encode(__binding_2, c, w)?;
                    }
                }
                Ok(())
            }
        }
        impl libipld::codec::Decode<libipld::cbor::DagCborCodec> for RootMap {
            fn decode<R: std::io::Read + std::io::Seek>(
                c: libipld::cbor::DagCborCodec,
                r: &mut R,
            ) -> libipld::Result<Self> {
                use libipld::cbor::decode::{read_len, read_u8};
                use libipld::cbor::error::{LengthOutOfRange, MissingKey, UnexpectedCode};
                use std::io::SeekFrom;
                let major = read_u8(r)?;
                match major {
                    0xa0..=0xbb => {
                        let len = read_len(r, major - 0xa0)?;
                        if len > 3usize {
                            return Err(LengthOutOfRange::new::<Self>().into());
                        }
                        let mut __binding_0 = None;
                        let mut __binding_1 = None;
                        let mut __binding_2 = None;
                        for _ in 0..len {
                            let key: String = Decode::decode(c, r)?;
                            match key.as_str() {
                                "entries" => {
                                    __binding_0 = Some(Decode::decode(c, r)?);
                                }
                                "lamport" => {
                                    __binding_1 = Some(Decode::decode(c, r)?);
                                }
                                "time" => {
                                    __binding_2 = Some(Decode::decode(c, r)?);
                                }
                                _ => {
                                    libipld::Ipld::decode(c, r)?;
                                }
                            }
                        }
                        let __binding_0 = __binding_0.ok_or(MissingKey::new::<Self>("entries"))?;
                        let __binding_1 = __binding_1.ok_or(MissingKey::new::<Self>("lamport"))?;
                        let __binding_2 = __binding_2.ok_or(MissingKey::new::<Self>("time"))?;
                        return Ok(RootMap {
                            entries: __binding_0,
                            lamport: __binding_1,
                            time: __binding_2,
                        });
                    }
                    0xbf => {
                        let mut __binding_0 = None;
                        let mut __binding_1 = None;
                        let mut __binding_2 = None;
                        loop {
                            let major = read_u8(r)?;
                            if major == 0xff {
                                break;
                            }
                            r.seek(SeekFrom::Current(-1))?;
                            let key = String::decode(c, r)?;
                            match key.as_str() {
                                "entries" => {
                                    __binding_0 = Some(Decode::decode(c, r)?);
                                }
                                "lamport" => {
                                    __binding_1 = Some(Decode::decode(c, r)?);
                                }
                                "time" => {
                                    __binding_2 = Some(Decode::decode(c, r)?);
                                }
                                _ => {
                                    libipld::Ipld::decode(c, r)?;
                                }
                            }
                        }
                        let __binding_0 = __binding_0.ok_or(MissingKey::new::<Self>("entries"))?;
                        let __binding_1 = __binding_1.ok_or(MissingKey::new::<Self>("lamport"))?;
                        let __binding_2 = __binding_2.ok_or(MissingKey::new::<Self>("time"))?;
                        return Ok(RootMap {
                            entries: __binding_0,
                            lamport: __binding_1,
                            time: __binding_2,
                        });
                    }
                    _ => {
                        return Err(UnexpectedCode::new::<Self>(major).into());
                    }
                }
            }
        }

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
                let new: super::RootMap = Arbitrary::arbitrary(g);
                Self {
                    entries: new.entries,
                    lamport: new.lamport,
                    time: new.time,
                }
            }
        }
        impl Arbitrary for RootUpdate {
            fn arbitrary(g: &mut quickcheck::Gen) -> Self {
                let new = super::RootUpdate::arbitrary(g);
                Self {
                    stream: new.stream,
                    root: new.root,
                    blocks: new.blocks,
                    lamport: new.lamport,
                    time: new.time,
                }
            }
        }
    }
}
