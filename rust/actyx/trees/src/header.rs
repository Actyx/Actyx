use crate::axtrees::Sha256Digest;
use actyxos_sdk::LamportTimestamp;
use libipld::{
    cbor::DagCborCodec,
    codec::{Decode, Encode},
    DagCbor,
};
use std::io;

/// Actyx tree header.
///
/// This is v0, which just contains a lamport timestamp. Later there will also be a signature.
#[derive(Debug, Clone)]
pub struct Header {
    pub root: Sha256Digest,
    pub lamport: LamportTimestamp,
}

impl Header {
    pub fn new(root: Sha256Digest, lamport: LamportTimestamp) -> Self {
        Self { root, lamport }
    }
}

impl Decode<DagCborCodec> for Header {
    fn decode<R: std::io::Read + std::io::Seek>(c: DagCborCodec, r: &mut R) -> anyhow::Result<Self> {
        HeaderIo::decode(c, r).map(Into::into)
    }
}

impl Encode<DagCborCodec> for Header {
    fn encode<W: io::Write>(&self, c: DagCborCodec, w: &mut W) -> anyhow::Result<()> {
        HeaderIo::from(self).encode(c, w)
    }
}

impl From<&Header> for HeaderIo {
    fn from(value: &Header) -> Self {
        HeaderIo::V1(value.root, value.lamport)
    }
}

impl From<HeaderIo> for Header {
    fn from(value: HeaderIo) -> Self {
        match value {
            HeaderIo::V1(root, lamport) => Self { root, lamport },
        }
    }
}

#[derive(DagCbor)]
#[ipld(repr = "int-tuple")]
enum HeaderIo {
    V1(Sha256Digest, LamportTimestamp),
}
