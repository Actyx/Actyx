//! this entire file used to be part of https://github.com/multiformats/rust-cid in 0.4,
//! but got removed in some refactoring. @vmxc copied it to the public rust-ipfs project,
//! so this is where I got it from.
use cid::{self, Cid, Version};
use multihash::{Code, MultihashDigest};
#[cfg(test)]
use quickcheck::{Arbitrary, Gen};
use std::convert::TryFrom;
use unsigned_varint::{decode as varint_decode, encode as varint_encode};

/// Prefix represents all metadata of a CID, without the actual content.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Prefix {
    /// The version of CID.
    pub version: Version,
    /// The codec of CID.
    pub codec: u64,
    /// The multihash type of CID.
    pub mh_type: u64,
    /// The multihash length of CID.
    pub mh_len: usize,
}

impl Prefix {
    /// Create a new prefix from encoded bytes.
    pub fn new(data: &[u8]) -> Result<Prefix, cid::Error> {
        let (raw_version, remain) = varint_decode::u64(data).map_err(|_| cid::Error::VarIntDecodeError)?;
        let version = Version::try_from(raw_version)?;

        let (codec, remain) = varint_decode::u64(remain).map_err(|_| cid::Error::VarIntDecodeError)?;

        let (mh_type, remain) = varint_decode::u64(remain).map_err(|_| cid::Error::VarIntDecodeError)?;

        let (mh_len, _remain) = varint_decode::usize(remain).map_err(|_| cid::Error::VarIntDecodeError)?;

        Ok(Prefix {
            version,
            codec,
            mh_type,
            mh_len,
        })
    }

    /// Convert the prefix to encoded bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut res = Vec::with_capacity(4);

        let mut buf = varint_encode::u64_buffer();
        let version = varint_encode::u64(self.version.into(), &mut buf);
        res.extend_from_slice(version);
        let mut buf = varint_encode::u64_buffer();
        let codec = varint_encode::u64(self.codec, &mut buf);
        res.extend_from_slice(codec);
        let mut buf = varint_encode::u64_buffer();
        let mh_type = varint_encode::u64(self.mh_type, &mut buf);
        res.extend_from_slice(mh_type);
        let mut buf = varint_encode::u64_buffer();
        let mh_len = varint_encode::u64(self.mh_len as u64, &mut buf);
        res.extend_from_slice(mh_len);

        res
    }

    /// Create a CID out of the prefix and some data that will be hashed
    pub fn to_cid(&self, data: &[u8]) -> Result<Cid, cid::Error> {
        let mut hash = Code::try_from(self.mh_type)?.digest(data);
        if self.mh_len < hash.digest().len() {
            hash = multihash::Multihash::wrap(hash.code(), &hash.digest()[..self.mh_len])?;
        }
        Cid::new(self.version, self.codec, hash)
    }
}

impl From<&Cid> for Prefix {
    fn from(cid: &Cid) -> Self {
        Self {
            version: cid.version(),
            codec: cid.codec(),
            mh_type: cid.hash().code(),
            mh_len: cid.hash().digest().len(),
        }
    }
}

#[cfg(test)]
impl Arbitrary for Prefix {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let cid: Cid = Arbitrary::arbitrary(g);
        Prefix {
            version: cid.version(),
            codec: cid.codec(),
            mh_type: cid.hash().code(),
            mh_len: cid.hash().digest().len(),
        }
    }
}
