use crate::EnvelopeList;
use libipld::error::Result;
use libipld::DagCbor;
use serde::{Deserialize, Serialize};

#[derive(DagCbor, Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "kebab-case")]
#[ipld(repr = "string")]
pub enum Compression {
    #[ipld(rename = "cbor-zstd")]
    CborZstd,
}

#[derive(DagCbor, Debug, PartialEq, Eq, Clone)]
#[ipld(repr = "kinded")]
pub enum Data {
    #[ipld(repr = "value")]
    List(Vec<u8>),
    #[ipld(repr = "value")]
    Bytes(Box<[u8]>),
}

impl AsRef<[u8]> for Data {
    fn as_ref(&self) -> &[u8] {
        match self {
            Data::List(b) => &b,
            Data::Bytes(b) => &b,
        }
    }
}

#[derive(Clone, DagCbor, PartialEq, Eq)]
pub struct Block {
    compression: Compression,
    data: Data,
}

impl Block {
    pub fn compress(compression: Compression, data: &EnvelopeList) -> Result<Self> {
        let decompressed = serde_cbor::to_vec(data)?;
        let mut compressed = Vec::new();
        zstd::stream::copy_encode(std::io::Cursor::new(decompressed), &mut compressed, 10)?;
        Ok(Self {
            compression,
            data: Data::Bytes(compressed.into_boxed_slice()),
        })
    }

    pub fn decompress(&self) -> Result<EnvelopeList> {
        let mut decompressed = Vec::new();
        zstd::stream::copy_decode(self.data.as_ref(), &mut decompressed)?;
        Ok(serde_cbor::from_slice(&decompressed)?)
    }
}

impl AsRef<[u8]> for Block {
    fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[Buffer: {} bytes]", self.data.as_ref().len())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::*;
    use actyxos_sdk::{fish_name, semantics, tags};
    use itertools::Itertools;
    use libipld::cbor::DagCborCodec;
    use libipld::codec::Codec;
    use serde_json::json;

    #[test]
    fn cbor_encode_decode_cbor_zstd() {
        let payload = (0..=10).map(|_| "0").join(",");
        let envelope = IpfsEnvelope {
            semantics: semantics!("semantics"),
            name: fish_name!("name"),
            tags: tags! { "foo", "bar" },
            timestamp: Timestamp::now(),
            offset: Offset::mk_test(1),
            payload: Payload::from_json_value(json!([payload])).unwrap(),
            lamport: LamportTimestamp::new(0),
        };
        let envelope_list = EnvelopeList::single(envelope);
        let block = Block::compress(Compression::CborZstd, &envelope_list).unwrap();
        let bytes = DagCborCodec.encode(&block).unwrap();
        let block2: Block = DagCborCodec.decode(&bytes).unwrap();
        assert_eq!(block2.decompress().unwrap(), block.decompress().unwrap());
    }

    #[test]
    fn encoded_zstd_test_small() -> Result<(), Box<dyn std::error::Error>> {
        // check that the cbor -> serde_cbor roundtrip works also with a real block
        let cbor_zstd_bytes = std::fs::read("test-data/encoded_zstd_test_small.cbor").unwrap();
        let expected: Block = DagCborCodec.decode(&cbor_zstd_bytes)?;
        let encoded_using_cbor = DagCborCodec.encode(&expected)?;
        let decoded_using_serde_cbor: Block = DagCborCodec.decode(&encoded_using_cbor)?;
        assert_eq!(decoded_using_serde_cbor, expected);
        Ok(())
    }

    /// the purpose of this test is to alert us when some detail about compression changes,
    /// e.g. due to a change in the zstd dependency
    #[test]
    fn encoded_zstd_test() -> Result<(), Box<dyn std::error::Error>> {
        // get one of the old strangely encoded blocks, it does not really matter for this test
        let cbor_zstd_bytes: Vec<u8> = std::fs::read("test-data/encoded_zstd_test.cbor")?;
        let block: Block = DagCborCodec.decode(&cbor_zstd_bytes)?;
        let evs1 = block.decompress().unwrap();
        let block = Block::compress(Compression::CborZstd, &evs1)?;
        let data_cbor_zstd = DagCborCodec.encode(&block)?;
        assert_eq!(data_cbor_zstd.len(), 49123);
        Ok(())
    }
}
