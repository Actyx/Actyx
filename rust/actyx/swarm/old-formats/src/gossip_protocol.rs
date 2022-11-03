//! Inlined encoding and decoding functions for the Gossip Protocol for Actyx <= 2.3.1 to
//! ensure backwards compatibility.
#![allow(clippy::all)]
use actyx_sdk::{LamportTimestamp, StreamId, Timestamp};
use anyhow::Result;
use ipfs_embed::{Block, Cid};
use libipld::{
    cbor::{decode::read_link, DagCborCodec as DagCbor14},
    codec::{Decode as Decode14, Encode as Encode14},
};
use libipld12::{
    cbor::{
        decode::{read_len, read_u8},
        encode::{write_tag, write_u64},
        error::{LengthOutOfRange, MissingKey, UnexpectedCode},
        DagCborCodec,
    },
    codec::{Decode, Encode},
    Ipld,
};
use std::io::{Read, Seek, SeekFrom, Write};
use std::{collections::BTreeMap, convert::TryFrom};
use trees::StoreParams;

struct CidOld(Cid);
impl Encode<DagCborCodec> for CidOld {
    fn encode<W: Write>(&self, _: DagCborCodec, w: &mut W) -> Result<()> {
        write_tag(w, 42)?;
        // insert zero byte per https://github.com/ipld/specs/blob/master/block-layer/codecs/dag-cbor.md#links
        // TODO: don't allocate
        let buf = self.0.to_bytes();
        let len = buf.len();
        write_u64(w, 2, len as u64 + 1)?;
        w.write_all(&[0])?;
        w.write_all(&buf[..len])?;
        Ok(())
    }
}
impl Decode<DagCborCodec> for CidOld {
    fn decode<R: Read + Seek>(_: DagCborCodec, r: &mut R) -> Result<Self> {
        let major = read_u8(r)?;
        if major == 0xd8 {
            if let Ok(tag) = read_u8(r) {
                if tag == 42 {
                    return read_link(r).map(CidOld);
                }
            }
        }
        Err(UnexpectedCode::new::<Self>(major).into())
    }
}

#[derive(PartialOrd, Ord, PartialEq, Eq)]
struct StreamIdOld(StreamId);
impl Encode<DagCborCodec> for StreamIdOld {
    fn encode<W: Write>(&self, _: DagCborCodec, w: &mut W) -> Result<()> {
        Encode14::encode(&self.0, DagCbor14, w)
    }
}
impl Decode<DagCborCodec> for StreamIdOld {
    fn decode<R: Read + Seek>(_: DagCborCodec, r: &mut R) -> Result<Self> {
        Decode14::decode(DagCbor14, r).map(Self)
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum GossipMessage {
    RootUpdate(RootUpdate),
    RootMap(RootMap),
}
impl Encode<DagCborCodec> for GossipMessage {
    fn encode<W: std::io::Write>(&self, c: DagCborCodec, w: &mut W) -> libipld::Result<()> {
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
impl Decode<DagCborCodec> for GossipMessage {
    fn decode<R: std::io::Read + std::io::Seek>(c: DagCborCodec, r: &mut R) -> libipld::Result<Self> {
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
pub struct RootUpdate {
    pub stream: StreamId,
    pub root: Cid,
    pub blocks: Vec<Block<StoreParams>>,
    pub lamport: LamportTimestamp,
    pub time: Timestamp,
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
impl Encode<DagCborCodec> for RootUpdateIo {
    fn encode<W: std::io::Write>(&self, c: DagCborCodec, w: &mut W) -> libipld::Result<()> {
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
                Encode14::encode(__binding_2, DagCbor14, w)?;
                Encode::encode("lamport", c, w)?;
                Encode14::encode(__binding_3, DagCbor14, w)?;
                Encode::encode("root", c, w)?;
                Encode14::encode(__binding_1, DagCbor14, w)?;
                Encode::encode("stream", c, w)?;
                Encode14::encode(__binding_0, DagCbor14, w)?;
                Encode::encode("time", c, w)?;
                Encode14::encode(__binding_4, DagCbor14, w)?;
            }
        }
        Ok(())
    }
}
impl Decode<DagCborCodec> for RootUpdateIo {
    fn decode<R: std::io::Read + std::io::Seek>(c: DagCborCodec, r: &mut R) -> libipld::Result<Self> {
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
                            __binding_2 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        "lamport" => {
                            __binding_3 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        "root" => {
                            __binding_1 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        "stream" => {
                            __binding_0 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        "time" => {
                            __binding_4 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        _ => {
                            Ipld::decode(c, r)?;
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
                    let key = <String as Decode<DagCborCodec>>::decode(c, r)?;
                    match key.as_str() {
                        "blocks" => {
                            __binding_2 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        "lamport" => {
                            __binding_3 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        "root" => {
                            __binding_1 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        "stream" => {
                            __binding_0 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        "time" => {
                            __binding_4 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        _ => {
                            Ipld::decode(c, r)?;
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
            .collect::<Result<Vec<Block<StoreParams>>>>()?;
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
pub struct RootMap {
    pub entries: BTreeMap<StreamId, Cid>,
    pub lamport: LamportTimestamp,
    pub time: Timestamp,
}
impl Encode<DagCborCodec> for RootMap {
    fn encode<W: std::io::Write>(&self, c: DagCborCodec, w: &mut W) -> libipld::Result<()> {
        match *self {
            RootMap {
                entries: ref __binding_0,
                lamport: ref __binding_1,
                time: ref __binding_2,
            } => {
                let len = 3u64;
                let entries = __binding_0
                    .iter()
                    .map(|(k, v)| (StreamIdOld(*k), CidOld(*v)))
                    .collect::<BTreeMap<_, _>>();
                write_u64(w, 5, len)?;
                Encode::encode("entries", c, w)?;
                Encode::encode(&entries, c, w)?;
                Encode::encode("lamport", c, w)?;
                Encode14::encode(__binding_1, DagCbor14, w)?;
                Encode::encode("time", c, w)?;
                Encode14::encode(__binding_2, DagCbor14, w)?;
            }
        }
        Ok(())
    }
}
impl Decode<DagCborCodec> for RootMap {
    fn decode<R: std::io::Read + std::io::Seek>(c: DagCborCodec, r: &mut R) -> libipld::Result<Self> {
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
                            __binding_0 = Some(
                                <BTreeMap<StreamIdOld, CidOld>>::decode(c, r)?
                                    .into_iter()
                                    .map(|(k, v)| (k.0, v.0))
                                    .collect(),
                            );
                        }
                        "lamport" => {
                            __binding_1 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        "time" => {
                            __binding_2 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        _ => {
                            Ipld::decode(c, r)?;
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
                    let key: String = Decode::decode(c, r)?;
                    match key.as_str() {
                        "entries" => {
                            __binding_0 = Some(
                                <BTreeMap<StreamIdOld, CidOld>>::decode(c, r)?
                                    .into_iter()
                                    .map(|(k, v)| (k.0, v.0))
                                    .collect(),
                            );
                        }
                        "lamport" => {
                            __binding_1 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        "time" => {
                            __binding_2 = Some(Decode14::decode(DagCbor14, r)?);
                        }
                        _ => {
                            Ipld::decode(c, r)?;
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
