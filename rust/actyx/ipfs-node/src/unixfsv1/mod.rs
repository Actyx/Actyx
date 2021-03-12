//! the purpose of this module is to deserialize raw ipfs unixfs v1 blobs and to traverse
//! recursive structures such as directories and files, returning a stream of raw bytes.
//!
//! Large files are supported. Extremely large directories are not yet supported
use crate::IpfsNode;
use anyhow::Result;
use futures::future::{BoxFuture, Future};
use futures::stream::Stream;
use libipld::cid::Cid;
use libipld::pb::{PbLink, PbNode};
use prost::Message;
use std::collections::{BTreeMap, VecDeque};
use std::pin::Pin;
use std::task::{Context, Poll};
use unixfs_pb::data::DataType;

mod unixfs_pb {
    include!(concat!(env!("OUT_DIR"), "/unixfs_pb.rs"));
}

/// The data of a unixfs v1 node that is needed for traversal
#[derive(Debug, Clone)]
enum UnixFsNode {
    /// A file leaf node, containing the raw data
    FileLeaf(Vec<u8>),
    /// a file branch node, containing an ordered sequence of child nodes
    FileBranch(Vec<Cid>),
    /// a directory node, containing a mapping from name to child cid
    Directory(BTreeMap<String, Cid>),
}

/// decodes a data block as unixfs v1
///
/// returns just the information relevant for traversal as an enum
fn decode_unixfs_block(block: &[u8]) -> Result<UnixFsNode> {
    // try to decode as a dag_pb IPLD object
    let dag = PbNode::from_bytes(block).map_err(|err| anyhow::anyhow!("{}", err))?;
    // the data must be a unixfs v1 protobuf, or it is not a proper unixfs v1 object
    let unixfs: unixfs_pb::Data = unixfs_pb::Data::decode(&*dag.data)?;
    // type must be a file
    if unixfs.r#type == DataType::File as i32 {
        if unixfs.blocksizes.is_empty() {
            // a leaf, data must be in the data field
            Ok(UnixFsNode::FileLeaf(
                unixfs.data.ok_or_else(|| anyhow::anyhow!("Missing field `Data`."))?,
            ))
        } else {
            // a branch, just parse the links as cids
            let mut cids: Vec<Cid> = Vec::with_capacity(dag.links.len());
            for PbLink { cid, .. } in dag.links {
                cids.push(cid);
            }
            Ok(UnixFsNode::FileBranch(cids))
        }
    } else if unixfs.r#type == DataType::Directory as i32 {
        let mut children: BTreeMap<String, Cid> = BTreeMap::new();
        for PbLink { cid, name, .. } in dag.links {
            children.insert(name, cid);
        }
        Ok(UnixFsNode::Directory(children))
    } else {
        Err(anyhow::anyhow!("Not an unixfs v1 object"))
    }
}

pub struct UnixfsDecoder {
    ipfs: IpfsNode,
    cids: VecDeque<Cid>,
    path: VecDeque<String>,
    index_html: bool,
}

impl UnixfsDecoder {
    pub fn new(ipfs: IpfsNode, cid: Cid, path: VecDeque<String>) -> Self {
        let mut cids = VecDeque::new();
        cids.push_back(cid);
        Self {
            ipfs,
            cids,
            path,
            index_html: false,
        }
    }

    /// decodes a tree of data blocks as an unixfs v1 file, traversing the links
    ///
    /// if successful, will return a stream of chunks. If unsuccessful, it might not terminate
    /// after the first unsuccessful chunk.
    pub async fn next(&mut self) -> Result<Option<Vec<u8>>> {
        loop {
            if let Some(cid) = self.cids.pop_front() {
                let cid = Cid::new_v1(cid.codec(), *cid.hash());
                let block = self.ipfs.fetch(&cid).await?;
                let node = decode_unixfs_block(block.data())?;
                if let Some(path) = self.path.pop_front() {
                    if let UnixFsNode::Directory(mut children) = node {
                        if let Some(cid) = children.remove(&path) {
                            self.cids.push_back(cid);
                        } else {
                            return Err(anyhow::anyhow!("expected child with name {}", path));
                        }
                    } else {
                        return Err(anyhow::anyhow!("expected a directory"));
                    }
                } else {
                    match node {
                        UnixFsNode::FileLeaf(data) => return Ok(Some(data)),
                        UnixFsNode::FileBranch(cids) => {
                            for cid in cids {
                                self.cids.push_back(cid);
                            }
                        }
                        UnixFsNode::Directory(mut children) => {
                            if self.index_html {
                                return Err(anyhow::anyhow!("expected `index.html` to be a file."));
                            } else {
                                self.index_html = true;
                                if let Some(cid) = children.remove("index.html") {
                                    self.cids.push_back(cid);
                                } else {
                                    return Err(anyhow::anyhow!("expected a file"));
                                }
                            }
                        }
                    }
                }
            } else {
                return Ok(None);
            }
        }
    }

    async fn owned_next(mut self) -> (Self, Result<Option<Vec<u8>>>) {
        let result = self.next().await;
        (self, result)
    }
}

pub struct UnixfsStream {
    future: BoxFuture<'static, (UnixfsDecoder, Result<Option<Vec<u8>>>)>,
}

impl UnixfsStream {
    pub fn new(decoder: UnixfsDecoder) -> Self {
        Self {
            future: Box::pin(decoder.owned_next()),
        }
    }
}

impl Stream for UnixfsStream {
    type Item = Result<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.future).poll(cx) {
            Poll::Ready((decoder, result)) => {
                self.future = Box::pin(decoder.owned_next());
                Poll::Ready(result.transpose())
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Block;
    use futures::stream::StreamExt;
    use libipld::multihash::{Code, MultihashDigest};
    use libipld::pb::DagPbCodec;
    use std::path::Path;

    fn setup_logger() {
        tracing_log::LogTracer::init().ok();
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .finish();
        tracing::subscriber::set_global_default(subscriber).ok();
    }

    const TEST_DATA: &[&str] = &[
        // small file
        "bafybeievd6mwe6vcwnkwo3eizs3h7w3a34opszbyfxziqdxguhjw7imdve",
        // large file
        "bafybeie572ii2oavoymyx4w2em2gvshop4iuqtr3kn5wo3ejl2oslld5lm",
        "bafybeibsr5kjsohzzjy5qvpycm27g3npukuluduozbmvywb6bdrpocmv7a",
        "bafybeifopef3jogtbfoyi3xieb3tntpurff54yzql3bx5reypk2v4p43ti",
        // directory
        "bafybeic7gluspmsogpwl5xz46ev2fyaoxai4q3tqk7okapeto76jfaud7a",
        "bafybeifecd74ibhpfk7mxdanj42dbd4jg76tzwvjbm7jixnzcwzb6qbvvi",
    ];

    async fn setup() -> IpfsNode {
        setup_logger();
        let client = IpfsNode::test().await.unwrap();
        for cid in TEST_DATA {
            let data = std::fs::read(Path::new("./test-data").join(cid)).unwrap();
            let hash = Code::Sha2_256.digest(&data);
            let cid = Cid::new_v1(DagPbCodec.into(), hash);
            let block = Block::new_unchecked(cid, data);
            client.insert(block).await.unwrap();
        }
        client
    }

    #[test]
    fn test_decode() -> Result<()> {
        // test the raw decoding of an individual file leaf block
        let raw: Vec<u8> = std::fs::read("./test-data/foo.raw").unwrap();
        let dag = PbNode::from_bytes(raw.as_slice())?;
        let data = dag.data;
        let unixfs: unixfs_pb::Data = unixfs_pb::Data::decode(&data[..])?;
        assert_eq!(unixfs.r#type, DataType::File as i32);
        assert_eq!(unixfs.data.unwrap(), b"foo\n");

        // test the raw decoding of an individual file branch block
        let raw: Vec<u8> = std::fs::read("./test-data/random500k.raw").unwrap();
        let dag = PbNode::from_bytes(raw.as_slice())?;
        let data = dag.data;
        let unixfs: unixfs_pb::Data = unixfs_pb::Data::decode(&data[..])?;
        assert_eq!(unixfs.r#type, DataType::File as i32);
        assert_eq!(unixfs.blocksizes, vec![262_144, 237_856]);

        // test the raw decoding of an individual directory block
        let raw: Vec<u8> = std::fs::read(format!("./test-data/{}", TEST_DATA[4])).unwrap();
        let dag = PbNode::from_bytes(raw.as_slice())?;
        let data = dag.data;
        let unixfs: unixfs_pb::Data = unixfs_pb::Data::decode(&data[..])?;
        assert_eq!(unixfs.r#type, DataType::Directory as i32);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn small_file_decode() -> Result<()> {
        // test the decoding of the entire file, which in this case just happens to be a
        // single block
        let ipfs = setup().await;
        let res = ipfs
            .cat(TEST_DATA[0].parse().unwrap(), Default::default())
            .map(|e| e.unwrap())
            .collect::<Vec<_>>()
            .await;
        assert_eq!(res, vec![b"foo\n".to_vec()]);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn large_file_decode() -> Result<()> {
        // test the tree traversal of the branch block and check that the children are as
        // big as the metadata says.
        let ipfs = setup().await;
        let res = ipfs
            .cat(TEST_DATA[1].parse().unwrap(), Default::default())
            .map(|x| x.unwrap().len())
            .collect::<Vec<_>>()
            .await;
        assert_eq!(res, vec![262_144, 237_856]);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn directory_decode() -> Result<()> {
        // test traversing a path from a complex directory
        let ipfs = setup().await;
        let cid = ipfs
            .cat(
                TEST_DATA[4].parse().unwrap(),
                ["chrome", "index.html"].iter().map(|s| s.to_string()).collect(),
            )
            .next()
            .await
            .unwrap()
            .unwrap_err()
            .downcast_ref::<libipld::error::BlockNotFound>()
            .unwrap()
            .0;
        assert_eq!(
            Cid::new_v0(*cid.hash()).unwrap(),
            "QmcTC7R6ho5afxLC4Jxx9gJWGvarDLu578JxreyQ41XhtC".parse::<Cid>().unwrap()
        );
        Ok(())
    }
}
