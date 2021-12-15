use actyx_sdk::{legacy::SourceId, tag, LamportTimestamp, NodeId, OffsetOrMin, Payload, StreamId, Tag, Timestamp};
use anyhow::{Context, Result};
use banyan::{
    store::{BlockWriter, BranchCache, ReadOnlyStore},
    Config, Forest, Transaction,
};
use ipfs_sqlite_block_store::{BlockStore, Synchronous};
use libipld::{
    cbor::DagCborCodec,
    codec::{Codec, Decode},
    Cid, DefaultParams, Link,
};
use parking_lot::Mutex;
use rayon::prelude::*;
use rusqlite::OpenFlags;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::TryFrom,
    fs,
    path::Path,
    str::FromStr,
    sync::Arc,
};
use trees::{
    axtrees::{AxKey, AxTrees, Sha256Digest},
    stags,
    tags::{ScopedTag, ScopedTagSet, TagScope},
    AxTree, AxTreeHeader,
};

use crate::sqlite_index_store::SqliteIndexStore;
use crate::{AxStreamBuilder, StreamAlias};

mod v1;
use v1::{Block, ConsNode, IpfsEnvelope};

type AxTxn<RW> = Transaction<AxTrees, RW, RW>;

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
            let block = get(db, &link)?;
            let envelopes = block.decompress()?;
            Ok(envelopes.into_vec())
        })
    })
}

fn envelope_to_v2(event: IpfsEnvelope, app_id: &str) -> (AxKey, Payload) {
    let mut tags = event.tags;
    tags.insert(tag!("semantics:") + event.semantics.as_str());
    tags.insert(tag!("fish_name:") + event.name.as_str());
    let mut tags: ScopedTagSet = tags.into();
    let app_id_tag = ScopedTag::new(
        TagScope::Internal,
        Tag::try_from(format!("app_id:{}", app_id).as_str()).unwrap(),
    );
    tags.insert(app_id_tag);
    let key: AxKey = AxKey::new(tags, event.lamport, event.timestamp);
    (key, event.payload)
}

/// in a transaction, convert an iterator of chunks of v1 events into a banyan tree
///
/// an ipfs blockstore transaction is not a db transation. It just protects the generated
/// stuff from gc.
#[allow(clippy::too_many_arguments)]
fn build_banyan_tree<'a, RW: BlockWriter<Sha256Digest> + ReadOnlyStore<Sha256Digest>>(
    txn: &'a AxTxn<RW>,
    source: &'a SourceId,
    stream_id: StreamId,
    iter: impl Iterator<Item = anyhow::Result<Vec<IpfsEnvelope>>> + Send + 'a,
    config: Config,
    app_id: &str,
    highest_lamport: Arc<Mutex<LamportTimestamp>>,
    emit_final_conversion_event: bool,
    earlier_version: u64,
    current_version: u64,
    own_node_id: NodeId,
) -> anyhow::Result<(AxTree, Vec<anyhow::Error>)> {
    let mut builder = AxStreamBuilder::new(config, Default::default());
    let mut last_offset = OffsetOrMin::MIN;
    let mut count = 0;
    let mut errors = Vec::new();
    let iter = iter
        .map(|r| {
            r.map(|envelopes| {
                count += envelopes.len();
                tracing::debug!("Building tree {} c={} e={}", source, count, errors.len());
                envelopes
            })
            .map_err(|e| {
                errors.push(e);
            })
            .unwrap_or_default()
        })
        .flatten()
        .flat_map(|e| {
            let offset = OffsetOrMin::from(e.offset);
            let diff = offset - last_offset;
            last_offset = offset;
            if diff > 1 {
                tracing::debug!(
                    "Encountered offset gap from offset {} to {}",
                    last_offset - diff.into(),
                    last_offset
                );
                let lamport = e.lamport;
                let timestamp = e.timestamp;
                Box::new((1..diff).into_iter().map(move |_| {
                    let mut tags = ScopedTagSet::empty();
                    let app_id_tag = ScopedTag::new(
                        TagScope::Internal,
                        Tag::try_from(format!("app_id:{}", app_id).as_str()).unwrap(),
                    );
                    tags.insert(app_id_tag);
                    let key: AxKey = AxKey::new(tags, lamport, timestamp);
                    (key, Payload::null())
                })) as Box<dyn Iterator<Item = (AxKey, Payload)> + Send>
            } else {
                Box::new(std::iter::empty())
            }
            .chain(std::iter::once(envelope_to_v2(e, app_id)))
        });
    txn.extend(&mut builder, iter)?;
    if emit_final_conversion_event {
        let ev = V1MigrationEvent {
            v1_source_id: *source,
            v2_stream_id: stream_id,
            writer: own_node_id,
            from: earlier_version,
            to: current_version,
        };
        let mut tags = stags!("migration");
        let app_id_tag = ScopedTag::new(
            TagScope::Internal,
            Tag::try_from(format!("app_id:{}", app_id).as_str()).unwrap(),
        );
        tags.insert(app_id_tag);

        let key = {
            let mut l = highest_lamport.lock();
            *l = l.incr();
            AxKey::new(tags, *l, Timestamp::now())
        };
        let payload = Payload::compact(&ev)?;
        txn.extend(&mut builder, std::iter::once((key, payload)))?;
    }
    Ok((builder.snapshot(), errors))
}
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct V1MigrationEvent {
    pub v1_source_id: SourceId,
    pub v2_stream_id: StreamId,
    pub writer: NodeId,
    pub from: u64,
    pub to: u64,
}

fn wrap_in_header<RW: BlockWriter<Sha256Digest> + ReadOnlyStore<Sha256Digest>>(
    txn: &AxTxn<RW>,
    root: Sha256Digest,
    lamport: LamportTimestamp,
) -> anyhow::Result<Sha256Digest> {
    // create the header
    let header = AxTreeHeader { root, lamport };
    // serialize it
    let header = DagCborCodec.encode(&header)?;
    // write it
    let root = txn.writer().put(header)?;
    Ok(root)
}

#[derive(Debug)]
pub struct V1IndexStoreInfo {
    pub roots: BTreeMap<SourceId, Cid>,
    pub lamport: LamportTimestamp,
}

/// Read a v1 index store and get all roots. Keys are source ids, not stream ids.
pub fn info_from_v1_index_store(path: impl AsRef<Path>) -> anyhow::Result<V1IndexStoreInfo> {
    let conn = rusqlite::Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut stmt = conn.prepare("SELECT source, cid FROM roots")?;
    let roots = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<Vec<(String, String)>>>()?;
    let roots = roots
        .into_iter()
        .map(|(source, cid)| Ok((SourceId::new(source)?, Cid::from_str(&cid)?)))
        .collect::<anyhow::Result<BTreeMap<_, _>>>()?;
    let mut stmt = conn.prepare("SELECT value FROM meta WHERE key = 'lamport'")?;
    let lamport: i64 = stmt.query_row([], |row| row.get(0)).unwrap_or(0);
    let lamport = LamportTimestamp::new(u64::try_from(lamport)?);
    Ok(V1IndexStoreInfo { roots, lamport })
}

fn write_index_store(
    v2_index_path: impl AsRef<Path>,
    lamport: LamportTimestamp,
    streams: impl Iterator<Item = StreamId>,
) -> anyhow::Result<()> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_FULL_MUTEX;
    let conn = rusqlite::Connection::open_with_flags(v2_index_path, flags)?;
    let conn = Arc::new(Mutex::new(conn));
    let mut store = SqliteIndexStore::from_conn(conn)?;
    store.received_lamport(lamport)?;
    for stream in streams {
        store.add_stream(stream)?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ConversionOptions {
    pub vacuum: bool,
    pub gc: bool,
    /// Sources to emit. None means all sources
    pub filtered_sources: Option<BTreeSet<SourceId>>,
    /// Mapping from SourceId to StreamId. If no entry is found, the StreamId
    /// will be created using the legacy conversion from the SourceId.
    pub source_to_stream: BTreeMap<SourceId, StreamId>,
}

impl Default for ConversionOptions {
    fn default() -> Self {
        Self {
            vacuum: true,
            gc: true,
            filtered_sources: None,
            source_to_stream: Default::default(),
        }
    }
}

/// Convert from an existing v1 actyx_data directory to an existing v2 actyx_directory for a given topic.
///
/// All files must already exist.
#[allow(clippy::too_many_arguments)]
pub fn convert_from_v1(
    v1_actyx_data: impl AsRef<Path>,
    v2_actyx_data: impl AsRef<Path>,
    topic: &str,
    app_id: &str,
    options: ConversionOptions,
    emit_final_conversion_event: bool,
    earlier_version: u64,
    current_version: u64,
    own_node_id: NodeId,
) -> anyhow::Result<()> {
    tracing::debug!("converting from v1 with opts: {:?}", options);
    let v1_blocks_path = v1_actyx_data.as_ref().join(format!("{}-blocks.sqlite", topic));
    let v1_index_path = v1_actyx_data.as_ref().join(topic);
    let v2_blocks_path = v2_actyx_data.as_ref().join(format!("store/{}.sqlite", topic));
    let v2_index_path = v2_actyx_data.as_ref().join("node.sqlite");
    anyhow::ensure!(
        fs::metadata(&v1_actyx_data)?.is_dir(),
        "source directory does not exist: {:?}",
        v1_actyx_data.as_ref().display()
    );
    anyhow::ensure!(
        fs::metadata(&v1_index_path)?.is_file(),
        "source index database does not exist: {:?}",
        v1_index_path
    );
    anyhow::ensure!(
        fs::metadata(&v1_blocks_path)?.is_file(),
        "source block database does not exist: {:?}",
        v1_blocks_path
    );
    anyhow::ensure!(
        fs::metadata(&v2_actyx_data)?.is_dir(),
        "target directory does not exist: {:?}",
        v2_actyx_data.as_ref().display()
    );
    anyhow::ensure!(
        fs::metadata(&v1_index_path)?.is_file(),
        "target index database does not exist: {:?}",
        v2_index_path
    );
    anyhow::ensure!(
        fs::metadata(&v1_blocks_path)?.is_file(),
        "target block database does not exist: {:?}",
        v2_blocks_path
    );
    tracing::debug!(
        "converting v1 files ({:?}, {:?}) to v2 files ({:?}, {:?})",
        v1_index_path,
        v1_blocks_path,
        v2_index_path,
        v2_blocks_path
    );

    tracing::debug!("opening existing v1 block store at {:?}", v1_blocks_path);
    let db1 = Arc::new(Mutex::new(BlockStore::open(
        v1_blocks_path,
        ipfs_sqlite_block_store::Config::default().with_pragma_synchronous(Synchronous::Normal),
    )?));
    let stats1 = db1.lock().get_store_stats()?;
    tracing::debug!("source block store stats at start of conversion {:?}", stats1);

    tracing::debug!("opening existing v2 block store at {:?}", v2_blocks_path);
    let db2 = Arc::new(Mutex::new(BlockStore::open(
        v2_blocks_path,
        ipfs_sqlite_block_store::Config::default().with_pragma_synchronous(Synchronous::Normal),
    )?));
    let stats2 = db2.lock().get_store_stats()?;
    tracing::debug!("target block store stats at start of conversion {:?}", stats2);

    let config = banyan::Config {
        max_leaf_count: 1 << 16,
        max_summary_branches: 32,
        max_key_branches: 32,
        target_leaf_size: 1 << 18,
        max_uncompressed_leaf_size: 1024 * 1024 * 4,
        zstd_level: 10,
    };
    let ss = Importer(db2.clone());
    let forest = Forest::new(ss, BranchCache::new(1 << 20));

    tracing::debug!("reading info from existing v1 index store at {:?}", v1_index_path);
    let info = info_from_v1_index_store(&v1_index_path).context("getting v1 db info")?;
    tracing::debug!("v1 info: {:?}", info);
    let lamport = Arc::new(Mutex::new(info.lamport));

    let result = info
        .roots
        .par_iter()
        .filter(|(source, _)| {
            options
                .filtered_sources
                .as_ref()
                .map(|f| f.contains(source))
                // None means all sources
                .unwrap_or(true)
        })
        .map(|(source, cid)| {
            let stream_id: StreamId = options
                .source_to_stream
                .get(source)
                .copied()
                // If there's no mapping, just convert
                .unwrap_or_else(|| source.into());
            tracing::debug!("converting tree {} ({})", source, stream_id);
            let txn = AxTxn::new(forest.clone(), forest.store().clone());
            let iter = iter_events_v1_chunked(&db1, Link::new(*cid));
            let tree = build_banyan_tree(
                &txn,
                source,
                stream_id,
                iter,
                config.clone(),
                app_id,
                lamport.clone(),
                emit_final_conversion_event,
                earlier_version,
                current_version,
                own_node_id,
            );

            match tree {
                Ok((tree, _errs)) => {
                    let root = tree
                        .link()
                        .map(|root| wrap_in_header(&txn, root, *lamport.lock()))
                        .transpose()?;
                    tracing::debug!("Setting alias {} {:?}", source, tree);
                    db2.lock()
                        .alias(StreamAlias::from(stream_id), root.map(Cid::from).as_ref())?;
                    Ok((source, stream_id, tree))
                }
                Err(cause) => {
                    tracing::error!("Error converting source {}: {}", source, cause);
                    Err(cause)
                }
            }
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    tracing::debug!("conversion done: {:?}", result);

    tracing::debug!("writing info to existing v2 index store at {:?}", v1_index_path);
    write_index_store(v2_index_path, *lamport.lock(), result.iter().map(|(_, s, _)| *s))?;

    if options.gc {
        tracing::debug!("running gc.");
        db2.lock().gc()?;
    }
    if options.vacuum {
        tracing::debug!("running vacuum.");
        db2.lock().vacuum()?;
    }

    let stats = db2.lock().get_store_stats()?;
    tracing::debug!("target block store stats at end of conversion {:?}", stats);

    Ok(())
}

#[derive(Clone)]
struct Importer(Arc<Mutex<BlockStore<crate::StoreParams>>>);

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

#[cfg(test)]
mod test {
    use actyx_sdk::{app_id, fish_name, semantics, source_id, Offset, TagSet};
    use banyan::{query::AllQuery, store::MemStore};

    use super::*;
    #[test]
    #[allow(clippy::needless_collect)] // It's not needless, clippy!
    fn should_fill_offset_gaps() -> anyhow::Result<()> {
        fn mk_envelope(offset: usize) -> IpfsEnvelope {
            IpfsEnvelope {
                lamport: (offset as u64).into(),
                name: fish_name!("name"),
                semantics: semantics!("sem"),
                offset: Offset::from(offset as u32),
                payload: Payload::from_json_str(&*format!("\"Non Empty String {}\"", offset)).unwrap(),
                tags: TagSet::empty(),
                timestamp: (offset as u64).into(),
            }
        }
        let v1_chunks: Vec<anyhow::Result<Vec<IpfsEnvelope>>> = vec![
            Ok(vec![]),
            Err(anyhow::anyhow!(":-(")),
            Ok((2..=5).into_iter().map(mk_envelope).collect()),
            Err(anyhow::anyhow!(":-/")),
            Ok((10..=11).into_iter().map(mk_envelope).collect()),
            Ok([12, 13, 15].iter().copied().map(mk_envelope).collect()),
        ];

        let offsets = v1_chunks
            .iter()
            .filter_map(|x| x.as_ref().ok())
            .flat_map(|x| x.iter())
            .map(|x| x.offset)
            .collect::<Vec<_>>();
        let max_idx = u64::from(*offsets.last().unwrap()) as usize + 1;
        let offsets_with_empty_payload = (0u64..max_idx as u64 - 1)
            .filter(|x| !offsets.contains(&Offset::from(*x as u32)))
            .collect::<Vec<_>>();

        let config = banyan::Config {
            max_leaf_count: 1 << 16,
            max_summary_branches: 32,
            max_key_branches: 32,
            target_leaf_size: 1 << 18,
            max_uncompressed_leaf_size: 1024 * 1024 * 4,
            zstd_level: 10,
        };

        let store = MemStore::new(usize::max_value(), Sha256Digest::new);
        let branch_cache = BranchCache::new(1000);
        let txn = AxTxn::new(Forest::new(store.clone(), branch_cache), store);
        let source = source_id!("v1_source_id");
        let stream_id = StreamId::from(source);
        let app_id = app_id!("com.actyx.from-v1");

        let (tree, errs) = build_banyan_tree(
            &txn,
            &source,
            stream_id,
            v1_chunks.into_iter(),
            config,
            &app_id,
            Default::default(),
            true,
            0,
            0,
            NodeId::from_bytes(&[0u8; 32]).unwrap(),
        )?;
        assert_eq!(errs.len(), 2);
        let x = txn.iter_filtered(&tree, AllQuery).collect::<anyhow::Result<Vec<_>>>()?;
        assert_eq!(x.len(), max_idx + 1);
        for (idx, (offset, key, payload)) in x.into_iter().enumerate() {
            assert_eq!(idx as u64, offset);
            assert_eq!(key.app_id(), Some(app_id.clone()));
            let tags = key.clone().into_tags();
            assert_eq!(
                tags.internal_tags().cloned().collect::<Vec<_>>(),
                vec![tag!("app_id:com.actyx.from-v1")]
            );
            if idx == max_idx {
                // conversion event
                let ev: V1MigrationEvent = payload.extract()?;
                assert_eq!(ev.v1_source_id, source);
                assert_eq!(ev.v2_stream_id, stream_id);
                assert_ne!(key.lamport(), offset.into());
                assert_ne!(key.time(), offset.into());

                assert_eq!(tags.public_tags().cloned().collect::<Vec<_>>(), vec![tag!("migration")]);
            } else if offsets_with_empty_payload.contains(&offset) {
                // filled gap
                assert_eq!(payload, Payload::null());
                assert!(tags.public_tags().next().is_none());
                assert_ne!(key.lamport(), offset.into());
                assert_ne!(key.time(), offset.into());
            } else {
                assert_eq!(key.lamport(), offset.into());
                assert_eq!(key.time(), offset.into());
                assert_eq!(payload.json_string(), format!("\"Non Empty String {}\"", offset));
                assert_eq!(
                    tags.public_tags().cloned().collect::<Vec<_>>(),
                    vec![tag!("fish_name:name"), tag!("semantics:sem")]
                );
            }
        }
        Ok(())
    }
}
