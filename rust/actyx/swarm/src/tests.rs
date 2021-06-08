use crate::{AxTreeExt, BanyanStore, MAX_TREE_LEVEL};
use actyx_sdk::{AppId, Offset, Payload, StreamNr, Tag, TagSet, app_id, tags};
use ax_futures_util::{prelude::AxStreamExt, stream::interval};
use banyan::query::AllQuery;
use futures::{prelude::*, StreamExt};
use libipld::Cid;
use std::{collections::BTreeMap, convert::TryFrom, str::FromStr, time::Duration};

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
async fn smoke() -> anyhow::Result<()> {
    util::setup_logger();
    let mut tagger = Tagger::new();
    let mut ev = move |tag| (tagger.tags(&[tag]), Payload::empty());
    let store = BanyanStore::test("smoke").await?;
    let ipfs = store.ipfs().clone();
    tokio::task::spawn(store.stream_filtered_stream_ordered(AllQuery).for_each(|x| {
        tracing::info!("got event {:?}", x);
        future::ready(())
    }));
    let stream_nr = StreamNr::try_from(1)?;
    tracing::info!("append first event!");
    let _ = store.append(stream_nr, app_id(), vec![ev("a")]).await?;
    tracing::info!("append second event!");
    tokio::task::spawn(interval(Duration::from_secs(1)).for_each(move |_| {
        let store = store.clone();
        let mut tagger = Tagger::new();
        let mut ev = move |tag| (tagger.tags(&[tag]), Payload::empty());
        async move {
            let _ = store.append(stream_nr, app_id(), vec![ev("a")]).await.unwrap();
        }
    }));
    tokio::task::spawn(ipfs.subscribe("test").unwrap().for_each(|msg| {
        tracing::error!("event {:?}", msg);
        future::ready(())
    }));
    tokio::time::sleep(Duration::from_secs(1000)).await;
    Ok(())
}

#[tokio::test]
async fn should_compact_regularly() -> anyhow::Result<()> {
    const EVENTS: usize = 10000;
    let store = BanyanStore::test("compaction_interval").await?;

    // Wait for the first compaction loop to pass.
    tokio::time::sleep(Duration::from_millis(500)).await;
    tokio::time::pause();

    let stream = store.get_or_create_own_stream(0.into())?;
    assert!(stream.published_tree().is_none());

    // Chunk to force creation of new branches
    for chunk in (0..EVENTS)
        .map(|_| (tags!("abc"), Payload::empty()))
        .collect::<Vec<_>>()
        .chunks(10)
        .into_iter()
    {
        store.append(0.into(), app_id(), chunk.to_vec()).await?;
    }
    let tree_after_append = stream.published_tree().unwrap();

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
    let tree_after_query = stream.published_tree().unwrap();
    assert_eq!(tree_after_append.root(), tree_after_query.root());
    assert!(!store.data.forest.is_packed(&tree_after_query.tree())?);

    tokio::time::advance(Duration::from_secs(60)).await;
    tokio::time::resume();
    // Wait here for compaction
    tokio::time::sleep(Duration::from_secs(1)).await;

    let tree_after_compaction = stream.published_tree().unwrap();
    assert!(tree_after_append.root() != tree_after_compaction.root());
    assert!(store.data.forest.is_packed(&tree_after_compaction.tree())?);
    Ok(())
}

#[tokio::test]
async fn should_extend_packed_when_hitting_max_tree_depth() -> anyhow::Result<()> {
    let store = BanyanStore::test("compaction_max_tree").await?;

    // Wait for the first compaction loop to pass.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let stream = store.get_or_create_own_stream(0.into())?;
    assert!(stream.published_tree().is_none());

    // Append individually to force creation of new branches
    for ev in (0..MAX_TREE_LEVEL + 1).map(|_| (tags!("abc"), Payload::empty())) {
        store.append(0.into(), app_id(), vec![ev]).await?;
    }
    let tree_after_append = stream.published_tree().unwrap().tree();
    assert!(!store.data.forest.is_packed(&tree_after_append)?);
    assert_eq!(tree_after_append.level(), MAX_TREE_LEVEL + 1);
    assert_eq!(
        tree_after_append.offset(),
        Some(Offset::try_from(MAX_TREE_LEVEL as i64).unwrap())
    );

    // packing will be triggered when the existing tree's level is MAX_TREE_LEVEL + 1
    store.append(0.into(), app_id(), vec![(tags!("abc"), Payload::empty())]).await?;
    let tree_after_pack = stream.published_tree().unwrap().tree();
    // the tree is not packed
    assert!(!store.data.forest.is_packed(&tree_after_pack)?);
    // but the max level remains constant now
    assert_eq!(tree_after_pack.level(), MAX_TREE_LEVEL + 1);
    assert_eq!(
        tree_after_pack.offset(),
        Some(Offset::try_from(MAX_TREE_LEVEL as i64 + 1).unwrap())
    );
    Ok(())
}
