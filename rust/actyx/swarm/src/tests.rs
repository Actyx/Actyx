use crate::{AxTreeExt, BanyanStore, SwarmConfig, MAX_TREE_LEVEL};
use acto::ActoRef;
use actyx_sdk::{app_id, tags, AppId, Offset, OffsetMap, Payload, StreamNr, Tag, TagSet};
use anyhow::Result;
use ax_futures_util::{
    prelude::AxStreamExt,
    stream::{interval, Drainer},
};
use banyan::query::AllQuery;
use futures::{pin_mut, prelude::*, StreamExt};
use libipld::Cid;
use maplit::btreemap;
use std::{collections::BTreeMap, convert::TryFrom, path::PathBuf, str::FromStr, time::Duration};
use tokio::runtime::Runtime;
use trees::query::TagExprQuery;

struct Tagger(BTreeMap<&'static str, Tag>);

impl Tagger {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn tag(&mut self, name: &'static str) -> Tag {
        self.0
            .entry(name)
            .or_insert_with(|| Tag::from_str(name).unwrap())
            .clone()
    }

    pub fn tags(&mut self, names: &[&'static str]) -> TagSet {
        names.iter().map(|name| self.tag(name)).collect::<TagSet>()
    }
}

fn app_id() -> AppId {
    app_id!("test")
}

#[allow(dead_code)]
fn cids_to_string(cids: Vec<Cid>) -> String {
    cids.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

#[tokio::test]
#[ignore]
async fn smoke() -> Result<()> {
    util::setup_logger();
    let mut tagger = Tagger::new();
    let mut ev = move |tag| (tagger.tags(&[tag]), Payload::null());
    let store = BanyanStore::test("smoke").await?;
    let mut ipfs = store.ipfs().clone();
    tokio::task::spawn(store.stream_filtered_stream_ordered(AllQuery).for_each(|x| {
        tracing::info!("got event {:?}", x);
        future::ready(())
    }));
    tracing::info!("append first event!");
    let _ = store.append(app_id(), vec![ev("a")]).await?;
    tracing::info!("append second event!");
    tokio::task::spawn(interval(Duration::from_secs(1)).for_each(move |_| {
        let store = store.clone();
        let mut tagger = Tagger::new();
        let mut ev = move |tag| (tagger.tags(&[tag]), Payload::null());
        async move {
            let _ = store.append(app_id(), vec![ev("a")]).await.unwrap();
        }
    }));
    tokio::task::spawn(async move {
        ipfs.subscribe("test".to_owned())
            .await
            .unwrap()
            .for_each(|msg| {
                tracing::error!("event {:?}", msg);
                future::ready(())
            })
            .await
    });
    tokio::time::sleep(Duration::from_secs(1000)).await;
    Ok(())
}

fn last_item<T: Clone>(drainer: &mut Drainer<T>) -> anyhow::Result<T> {
    let mut vec = drainer.next().ok_or_else(|| anyhow::anyhow!("Stream ended"))?;
    vec.pop().ok_or_else(|| anyhow::anyhow!("Stream returned pending"))
}

#[tokio::test]
async fn should_compact() -> Result<()> {
    // this will take 1010 chunks, so it will hit the MAX_TREE_LEVEL limit once
    const EVENTS: usize = 10100;
    let mut config = SwarmConfig::test("compaction_interval");
    config.cadence_compact = Duration::from_secs(100000);
    let store = BanyanStore::new(config, ActoRef::blackhole()).await?;

    // Wait for the first compaction loop to pass.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let stream = store.get_or_create_own_stream(0.into())?;
    let tree_stream = stream.tree_stream();
    let mut tree_stream = Drainer::new(tree_stream);
    assert_eq!(last_item(&mut tree_stream)?.count(), 0);

    // Chunk to force creation of new branches
    for chunk in (0..EVENTS)
        .map(|_| (tags!("abc"), Payload::null()))
        .collect::<Vec<_>>()
        .chunks(10)
    {
        store.append(app_id(), chunk.to_vec()).await?;
    }
    let tree_after_append = last_item(&mut tree_stream)?;
    assert!(!store.data.forest.is_packed(&tree_after_append)?);

    // get the events back
    let evs = store
        .stream_filtered_chunked(store.node_id().stream(0.into()), 0..=u64::MAX, AllQuery)
        .take_until_signaled(tokio::time::sleep(Duration::from_secs(2)))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .flat_map(|c| c.data);
    assert_eq!(evs.count(), EVENTS);
    // Make sure the root didn't change
    let empty = tree_stream.next().unwrap();
    assert!(empty.is_empty(), "{:?}", empty);

    // running compaction manually here to make the test deterministic
    let mut guard = stream.lock().await;
    store.transform_stream(&mut guard, |txn, tree| txn.pack(tree))?;
    drop(guard);

    let tree_after_compaction = last_item(&mut tree_stream)?;
    assert_ne!(tree_after_append.root(), tree_after_compaction.root());
    assert!(store.data.forest.is_packed(&tree_after_compaction)?);
    Ok(())
}

#[tokio::test]
async fn should_extend_packed_when_hitting_max_tree_depth() -> Result<()> {
    let store = BanyanStore::test("compaction_max_tree").await?;

    // Wait for the first compaction loop to pass.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let tree_stream = store.get_or_create_own_stream(0.into())?.tree_stream();
    let mut tree_stream = Drainer::new(tree_stream);
    assert_eq!(last_item(&mut tree_stream)?.count(), 0);

    // Append individually to force creation of new branches
    for ev in (0..MAX_TREE_LEVEL).map(|_| (tags!("abc"), Payload::null())) {
        store.append(app_id(), vec![ev]).await?;
    }
    let tree_after_append = last_item(&mut tree_stream)?;
    assert!(!store.data.forest.is_packed(&tree_after_append)?);
    assert_eq!(tree_after_append.level(), MAX_TREE_LEVEL);
    assert_eq!(
        tree_after_append.offset(),
        Some(Offset::try_from((MAX_TREE_LEVEL - 1) as i64).unwrap())
    );

    // packing will be triggered when the existing tree's level is MAX_TREE_LEVEL + 1
    store.append(app_id(), vec![(tags!("abc"), Payload::null())]).await?;
    let tree_after_pack = last_item(&mut tree_stream)?;
    // the tree is not packed
    assert!(store.data.forest.is_packed(&tree_after_pack)?);
    // but the max level remains constant now
    assert_eq!(tree_after_pack.level(), 3);
    assert_eq!(
        tree_after_pack.offset(),
        Some(Offset::try_from(MAX_TREE_LEVEL as i64).unwrap())
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn must_not_lose_events_through_compaction() -> Result<()> {
    const EVENTS: usize = 1000;
    let store = BanyanStore::test("compaction_max_tree").await?;
    // compact continuously
    store.spawn_task(
        "compaction".to_owned(),
        store.clone().compaction_loop(Duration::from_micros(0)),
    );

    let tags_query =
        TagExprQuery::from_expr(&"'abc'".parse().unwrap()).unwrap()(true, store.node_id().stream(0.into()));

    for ev in (0..EVENTS).map(|_| (tags!("abc"), Payload::null())) {
        store.append(app_id(), vec![ev]).await?;
    }

    let evs = store
        .stream_filtered_stream_ordered(tags_query)
        .take(EVENTS)
        .take_until_signaled(tokio::time::sleep(Duration::from_secs(2)))
        .map_ok(|x| x.0)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;
    anyhow::ensure!(
        evs.len() == EVENTS,
        "Expected {} events, but only got back {}. Received: {:?}",
        EVENTS,
        evs.len(),
        evs
    );

    Ok(())
}

fn config_in_temp_folder() -> anyhow::Result<(SwarmConfig, tempfile::TempDir)> {
    let dir = tempfile::tempdir()?;
    let db = PathBuf::from(dir.path().join("db").to_str().expect("illegal filename"));
    let index = PathBuf::from(dir.path().join("index").to_str().expect("illegal filename"));
    let config = SwarmConfig {
        index_store: Some(index),
        node_name: Some("must_report_proper_initial_offsets".to_owned()),
        db_path: Some(db),
        ..SwarmConfig::basic()
    };
    Ok((config, dir))
}

#[tokio::test]
async fn must_report_proper_initial_offsets() -> anyhow::Result<()> {
    const EVENTS: usize = 10;
    let (config, _dir) = config_in_temp_folder()?;
    let store = BanyanStore::new(config.clone(), ActoRef::blackhole()).await?;
    let stream_id = store.node_id().stream(0.into());
    let expected_present = OffsetMap::from(btreemap! { stream_id => Offset::from(9) });

    for ev in (0..EVENTS).map(|_| (tags!("abc"), Payload::null())) {
        store.append(app_id(), vec![ev]).await?;
    }

    let present = store.data.offsets.project(|x| x.present.clone());
    assert_eq!(present, expected_present);
    drop(store);

    // load non-empty store from disk and check that the offsets are correctly computed
    let store = BanyanStore::new(config, ActoRef::blackhole()).await?;
    let swarm_offsets = store.data.offsets.project(Clone::clone);
    assert_eq!(swarm_offsets.present, expected_present);
    // replication_target should be equal to the present. is nulled in the event service API
    assert_eq!(swarm_offsets.replication_target, expected_present);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_add_cat() -> Result<()> {
    use rand::RngCore;
    util::setup_logger();
    let store = BanyanStore::test("local").await?;
    let mut data = vec![0; 16_000_000];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut data);
    let mut tmp = store.ipfs().create_temp_pin()?;
    let (root, _) = store.add(&mut tmp, &data[..])?;
    let mut buf = Vec::with_capacity(16_000_000);
    let stream = store.cat(root, true);
    pin_mut!(stream);
    while let Some(res) = stream.next().await {
        let mut bytes = res?;
        buf.append(&mut bytes);
    }
    assert_eq!(buf, data);
    Ok(())
}

#[test]
fn test_add_zero_bytes() -> Result<()> {
    let rt = Runtime::new()?;
    rt.block_on(async {
        util::setup_logger();
        let store = BanyanStore::test("local").await?;
        tracing::info!("store created");
        let mut tmp = store.ipfs().create_temp_pin()?;
        tracing::info!("temp pin created");
        let data: &[u8] = &[];
        store.add(&mut tmp, data)?;
        tracing::info!("data added");
        drop(tmp);
        tracing::info!("temp pin dropped");
        drop(store); // without this the test sometimes doesn’t complete
        tracing::info!("store dropped");
        Ok(())
    })
}
