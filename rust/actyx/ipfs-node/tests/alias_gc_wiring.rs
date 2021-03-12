use anyhow::Result;
use ipfs_node::{Block, Cid, IpfsNode};
use libipld::multihash::{Code, MultihashDigest};

fn create_block<T: AsRef<[u8]>>(data: T) -> Block {
    let hash = Code::Sha2_256.digest(data.as_ref());
    let cid = Cid::new_v1(libipld::raw::RawCodec.into(), hash);
    Block::new_unchecked(cid, data.as_ref().to_vec())
}

/// The block store has its own tests. The purpose of this test is just to make sure that the store
/// is properly wired up.
#[tokio::test]
#[ignore]
async fn alias_gc_wiring() -> Result<()> {
    let node = IpfsNode::test().await?;
    let block1 = create_block(b"foo");
    let block2 = create_block(b"bar");
    node.insert(block1.clone()).await?;
    node.insert(block2.clone()).await?;
    node.alias_many(vec![(b"alias1".to_vec(), Some(*block1.cid()))]).await?;
    node.gc().await?;
    assert_eq!(node.fetch(block1.cid()).await?.data(), &b"foo"[..]);
    assert!(node.fetch(block2.cid()).await.is_err());
    node.alias_many(vec![(b"alias1".to_vec(), None)]).await?;
    node.gc().await?;
    assert!(node.fetch(block1.cid()).await.is_err());
    Ok(())
}
