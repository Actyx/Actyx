use std::{collections::BTreeMap, fs, str::FromStr, sync::Arc};

use actyxos_sdk::{legacy::SourceId, tag, Payload, StreamId};
use anyhow::Result;
use banyan::store::{BlockWriter, ReadOnlyStore};
use banyan::{store::BranchCache, Config, Forest, Transaction};
use ipfs_sqlite_block_store::{BlockStore, Synchronous};
use libipld::{
    cbor::DagCborCodec,
    codec::{Codec, Decode},
    Cid, DefaultParams, Link,
};
use parking_lot::Mutex;
use rayon::prelude::*;
use rusqlite::OpenFlags;
use trees::{
    axtrees::{AxKey, AxTrees, Sha256Digest},
    AxTree,
};

use crate::{AxStreamBuilder, StreamAlias};

mod v1;
use v1::{Block, ConsNode, IpfsEnvelope};

#[cfg(test)]
mod tests;

type AxTxn = Transaction<AxTrees, Importer, Importer>;

/// Get a block from a block store and decode it, with extensive logging
fn get<T: Decode<DagCborCodec>>(db: &Arc<Mutex<BlockStore<DefaultParams>>>, link: &Link<T>) -> anyhow::Result<T> {
    if let Some(data) = db.lock().get_block(link.cid())? {
        Ok(DagCborCodec.decode::<T>(&data).map_err(|e| {
            // log decode errors at error level, including a hexdump of the block
            // this can be copied in to cbor.me to figure out what is going on.
            tracing::error!("Link could not be decoded");
            tracing::error!("{}", link.cid());
            tracing::error!("{}", hex::encode(&data));
            tracing::error!("{}", e);
            e
        })?)
    } else {
        Err(anyhow::anyhow!("block not found in local db"))
    }
}

fn iter_chain(
    db: &Arc<Mutex<BlockStore<DefaultParams>>>,
    root: Link<ConsNode>,
) -> impl Iterator<Item = anyhow::Result<Link<Block>>> + '_ {
    itertools::unfold(Some(root), move |prev| {
        if let Some(link) = prev {
            Some(match get(db, link) {
                Ok(cons_node) => {
                    *prev = cons_node.prev;
                    Ok(cons_node.block)
                }
                Err(cause) => {
                    *prev = None;
                    Err(cause)
                }
            })
        } else {
            None
        }
    })
}

/// Iterate over a v1 chain
///
/// This will try to get as many events as possible, even if some blocks are missing.
#[allow(clippy::needless_collect)]
fn iter_events_v1_chunked(
    db: &Arc<Mutex<BlockStore<DefaultParams>>>,
    root: Link<ConsNode>,
) -> impl Iterator<Item = anyhow::Result<Vec<IpfsEnvelope>>> + '_ {
    let block_links = iter_chain(db, root).collect::<Vec<_>>();
    block_links.into_iter().rev().map(move |r| {
        r.and_then(|link| {
            let block = get(&db, &link)?;
            let envelopes = block.decompress()?;
            Ok(envelopes.into_vec())
        })
    })
}

/// Converts a block of events from v1 to v2
fn events_to_v2(envelopes: Vec<IpfsEnvelope>) -> Vec<(AxKey, Payload)> {
    envelopes
        .into_iter()
        .map(|event| {
            let mut tags = event.tags;
            tags.insert(tag!("semantics:") + event.semantics.as_str());
            tags.insert(tag!("fish_name:") + event.name.as_str());
            let tags = tags.into();
            let key: AxKey = AxKey::new(tags, event.lamport, event.timestamp);
            (key, event.payload)
        })
        .collect::<Vec<_>>()
}

/// in a transaction, convert an iterator of chunks of v1 events into a banyan tree
///
/// an ipfs blockstore transaction is not a db transation. It just protects the generated
/// stuff from gc.
fn build_banyan_tree<'a>(
    txn: &'a AxTxn,
    source: &'a SourceId,
    iter: impl Iterator<Item = anyhow::Result<Vec<IpfsEnvelope>>> + Send + 'a,
    config: Config,
) -> anyhow::Result<AxTree> {
    let mut builder = AxStreamBuilder::new(config, Default::default());
    let mut count: usize = 0;
    let mut errors = Vec::new();
    let iter = iter
        .map(|r| {
            r.map(|envelopes| {
                count += envelopes.len();
                tracing::debug!("Building tree {} c={} e={}", source, count, errors.len());
                events_to_v2(envelopes)
            })
            .map_err(|e| {
                errors.push(e.to_string());
                e
            })
            .unwrap_or_default()
        })
        .flatten();
    txn.extend(&mut builder, iter)?;
    Ok(builder.snapshot())
}

/// Read a v1 index store and get all roots. Keys are source ids, not stream ids.
fn roots_from_v1_store(path: &str) -> anyhow::Result<BTreeMap<SourceId, Cid>> {
    let conn = rusqlite::Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare("SELECT source, cid FROM roots")?;
    let res = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<Vec<(String, String)>>>()?;
    let res = res
        .into_iter()
        .map(|(source, cid)| Ok((SourceId::new(source)?, Cid::from_str(&cid)?)))
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
    Ok(res)
}

#[derive(Debug, Clone)]
pub struct ConversionOptions {
    pub vacuum: bool,
    pub gc: bool,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self { vacuum: true, gc: true }
    }
}

pub fn convert_from_v1(v1_index_path: &str, v2_index_path: &str, options: ConversionOptions) -> anyhow::Result<()> {
    let v1_blocks_path = format!("{}-blocks.sqlite", v1_index_path);
    let v2_blocks_path = format!("{}-blocks.sqlite", v2_index_path);

    anyhow::ensure!(
        fs::metadata(&v2_index_path).is_err(),
        "target file {} already exists!",
        v2_index_path
    );
    anyhow::ensure!(
        fs::metadata(&v2_blocks_path).is_err(),
        "target file {} already exists!",
        v2_blocks_path
    );
    tracing::info!("copying block store from {} to {}", v1_blocks_path, v2_blocks_path);
    let n = fs::copy(&v1_blocks_path, &v2_blocks_path)?;
    tracing::info!("copied {} bytes", n);
    let roots = roots_from_v1_store(&v1_index_path)?;

    let db = Arc::new(Mutex::new(BlockStore::open(
        v2_blocks_path,
        ipfs_sqlite_block_store::Config::default().with_pragma_synchronous(Synchronous::Normal),
    )?));
    let stats = db.lock().get_store_stats()?;
    tracing::info!("Block store stats at start of conversion {:?}", stats);

    let config = banyan::Config {
        max_leaf_count: 1 << 16,
        max_summary_branches: 32,
        max_key_branches: 32,
        target_leaf_size: 1 << 18,
        max_uncompressed_leaf_size: 1024 * 1024 * 4,
        zstd_level: 10,
    };
    let ss = Importer(db.clone());
    let forest = Forest::new(ss, BranchCache::new(1024));

    let _result = roots
        .par_iter()
        .map(|(source, cid)| {
            tracing::info!("converting tree {}", source);
            let txn = AxTxn::new(forest.clone(), forest.store().clone());
            let iter = iter_events_v1_chunked(&db, Link::new(*cid));
            let tree = build_banyan_tree(&txn, &source, iter, config.clone());
            match tree {
                Ok(tree) => {
                    tracing::info!("Setting alias {} {:?}", source, tree);
                    let stream_id: StreamId = source.into();
                    db.lock()
                        .alias(StreamAlias::from(stream_id), tree.link().map(Cid::from).as_ref())?;
                    Ok((source, tree))
                }
                Err(cause) => {
                    tracing::error!("Error converting source {}: {}", source, cause);
                    Err(cause)
                }
            }
        })
        .collect::<Vec<anyhow::Result<_>>>();
    tracing::info!("conversion done.");
    if options.gc {
        tracing::info!("running gc.");
        db.lock().gc()?;
    }
    if options.vacuum {
        tracing::info!("running vacuum.");
        db.lock().vacuum()?;
    }

    let stats = db.lock().get_store_stats()?;
    tracing::info!("Block store stats at end of conversion {:?}", stats);

    Ok(())
}

#[derive(Clone)]
struct Importer(Arc<Mutex<BlockStore<DefaultParams>>>);

impl ReadOnlyStore<Sha256Digest> for Importer {
    fn get(&self, link: &Sha256Digest) -> Result<Box<[u8]>> {
        let cid = Cid::from(*link);
        if let Some(data) = self.0.lock().get_block(&cid)? {
            Ok(data.into_boxed_slice())
        } else {
            Err(anyhow::anyhow!("block not found"))
        }
    }
}

impl BlockWriter<Sha256Digest> for Importer {
    fn put(&self, data: Vec<u8>) -> Result<Sha256Digest> {
        let digest = Sha256Digest::new(&data);
        let cid = digest.into();
        let block = crate::Block::new_unchecked(cid, data);
        self.0.lock().put_block(&block, None)?;
        Ok(digest)
    }
}
