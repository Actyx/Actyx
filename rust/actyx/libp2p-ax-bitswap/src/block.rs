//! Block
use cid::Cid;
use std::{
    cmp::{Ord, Ordering, PartialOrd},
    hash::{Hash, Hasher},
    sync::Arc,
};

#[derive(Clone, Debug, Eq)]
/// An immutable ipfs block.
pub struct Block {
    data: Arc<[u8]>,
    cid: Cid,
}

impl Block {
    pub const CBOR_TAG_LINK: u64 = 42;

    /// Creates a new immutable ipfs block.
    pub fn new(data: Vec<u8>, cid: Cid) -> Self {
        Block::from_arc(data.into(), cid)
    }

    pub fn from_arc(data: Arc<[u8]>, cid: Cid) -> Self {
        Block { data, cid }
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn cid(&self) -> &Cid {
        &self.cid
    }

    pub fn rough_size(&self) -> usize {
        self.cid.hash().digest().len() + 4 + self.data.len()
    }

    pub fn into_data(self) -> Arc<[u8]> {
        self.data
    }
}

impl Hash for Block {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.cid, state)
    }
}

impl PartialEq for Block {
    fn eq(&self, other: &Self) -> bool {
        self.cid == other.cid
    }
}

impl PartialOrd for Block {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cid.cmp(&other.cid))
    }
}

impl Ord for Block {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cid.cmp(&other.cid)
    }
}

#[cfg(test)]
mod tests {
    use super::Block;
    use crate::codecs::{DAG_PROTOBUF, RAW};
    use crate::prefix::Prefix;
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for Block {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let mut prefix: Prefix = Arbitrary::arbitrary(g);
            let data: Vec<u8> = Arbitrary::arbitrary(g);
            prefix.mh_type = multihash::Code::Sha2_256.into();
            let cid = prefix.to_cid(&data).unwrap();
            Block { data: data.into(), cid }
        }
    }

    #[test]
    fn test_raw_block_cid() {
        let content = b"hello\n";
        let cid = "bafkreicysg23kiwv34eg2d7qweipxwosdo2py4ldv42nbauguluen5v6am";
        let prefix = Prefix {
            version: cid::Version::V1,
            codec: RAW,
            mh_type: multihash::Code::Sha2_256.into(),
            mh_len: 32,
        };
        let computed_cid = prefix.to_cid(content).unwrap().to_string();
        assert_eq!(cid, computed_cid);
    }

    #[test]
    fn test_dag_pb_block_cid() {
        let content = b"hello\n";
        let cid = "QmUJPTFZnR2CPGAzmfdYPghgrFtYFB6pf1BqMvqfiPDam8";
        let prefix = Prefix {
            version: cid::Version::V0,
            codec: DAG_PROTOBUF,
            mh_type: multihash::Code::Sha2_256.into(),
            mh_len: 32,
        };
        let computed_cid = prefix.to_cid(content).unwrap().to_string();
        assert_eq!(cid, computed_cid);
    }
}
