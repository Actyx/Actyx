use crate::BanyanStore;
use actyx_sdk::{tags, Payload, StreamNr, Tag, TagSet};
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
    let _ = store.append(stream_nr, vec![ev("a")]).await?;
    tracing::info!("append second event!");
    tokio::task::spawn(interval(Duration::from_secs(1)).for_each(move |_| {
        let store = store.clone();
        let mut tagger = Tagger::new();
        let mut ev = move |tag| (tagger.tags(&[tag]), Payload::empty());
        async move {
            let _ = store.append(stream_nr, vec![ev("a")]).await.unwrap();
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
    let store = BanyanStore::test("compaction").await?;
    let start = tokio::time::Instant::now();

    // Wait for the first compaction loop to pass.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let stream = store.get_or_create_own_stream(0.into())?;
    assert!(stream.published_tree().is_none());

    // Chunk to force creation of new leaves
    for chunk in (0..EVENTS)
        .map(|_| (tags!("abc"), Payload::empty()))
        .collect::<Vec<_>>()
        .chunks(10)
        .into_iter()
    {
        store.append(0.into(), chunk.to_vec()).await?;
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

    // Wait here for compaction (or make it configurable in [`SwarmConfig`] ..)
    tokio::time::sleep_until(start + Duration::from_secs(61)).await;

    let tree_after_compaction = stream.published_tree().unwrap();
    assert!(tree_after_append.root() != tree_after_compaction.root());
    assert!(store.data.forest.is_packed(&tree_after_compaction.tree())?);
    Ok(())
}
