use ipfs_node::Block;
use libipld::{cbor::DagCborCodec, codec::Codec};
use libipld::{Cid, DagCbor};
use multihash::{Code, MultihashDigest};
use rand::{random, Rng};

/// creates a block
/// leaf blocks will be larger than branch blocks
pub fn block(name: &str, links: impl IntoIterator<Item = cid::Cid>) -> Block {
    let links = links.into_iter().collect::<Vec<_>>();
    let data_size = if links.is_empty() { 1024 * 16 - 16 } else { 512 };
    let mut name = name.to_string();
    while name.len() < data_size {
        name += " ";
    }
    let ipld = Node::branch(&name, links);
    let bytes = DagCborCodec.encode(&ipld).unwrap();
    let hash = Code::Sha2_256.digest(&bytes);
    // https://github.com/multiformats/multicodec/blob/master/table.csv
    Block::new_unchecked(Cid::new_v1(0x71, hash), bytes)
}

pub fn random_block() -> Block {
    let mut rnd = rand::thread_rng();
    let mut data = random::<[u8; 32]>();
    rnd.fill(&mut data);
    let data = serde_cbor::to_vec(&serde_cbor::Value::Bytes(data.to_vec())).unwrap();
    let hash = Code::Sha2_256.digest(&data);
    let cid = Cid::new_v1(DagCborCodec.into(), hash);
    Block::new_unchecked(cid, data)
}

fn build_tree_0(prefix: &str, branch: u64, depth: u64, blocks: &mut Vec<Block>) -> anyhow::Result<Cid> {
    let children = if depth == 0 {
        Vec::new()
    } else {
        let mut children = Vec::new();
        for i in 0..branch {
            let cid = build_tree_0(&format!("{}-{}", prefix, i), branch, depth - 1, blocks)?;
            children.push(cid);
        }
        children
    };
    let block = block(prefix, children);
    let cid = *block.cid();
    blocks.push(block);
    Ok(cid)
}

pub fn build_tree(prefix: &str, branch: u64, depth: u64) -> anyhow::Result<(Cid, Vec<Block>)> {
    let mut tmp = Vec::new();
    let res = build_tree_0(prefix, branch, depth, &mut tmp)?;
    Ok((res, tmp))
}

pub fn build_chain(prefix: &str, n: usize) -> anyhow::Result<(Cid, Vec<Block>)> {
    assert!(n > 0);
    let mut blocks = Vec::with_capacity(n);
    let mk_node = |i: usize, links| block(&format!("{}-{}", prefix, i), links);
    let mut prev: Option<Cid> = None;
    for i in 0..n {
        let node = mk_node(i, prev);
        prev = Some(*node.cid());
        blocks.push(node);
    }
    Ok((prev.unwrap(), blocks))
}

#[derive(Debug, DagCbor)]
struct Node {
    links: Vec<Cid>,
    text: String,
}

impl Node {
    pub fn branch(text: &str, links: impl IntoIterator<Item = Cid>) -> Self {
        Self {
            links: links.into_iter().collect(),
            text: text.into(),
        }
    }
}
