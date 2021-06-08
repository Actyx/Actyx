use std::{collections::BTreeMap, convert::TryFrom, str::FromStr, time::Duration};

use crate::BanyanStore;
use actyx_sdk::{tags, Payload, StreamNr, Tag, TagSet};
use ax_futures_util::{prelude::AxStreamExt, stream::interval};
use banyan::query::AllQuery;
use futures::{prelude::*, StreamExt};
use libipld::Cid;

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
async fn should_compact() -> anyhow::Result<()> {
    let store = BanyanStore::test("compaction").await?;

    // Wait for the first compaction loop to pass.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let stream = store.get_or_create_own_stream(0.into())?;
    assert!(stream.published_tree().is_none());
    store
        .append(
            0.into(),
            (0..1000).map(|_| (tags!("abc"), Payload::empty())).collect::<Vec<_>>(),
        )
        .await?;
    let root_after_append = stream.published_tree().unwrap().root();

    // get the events back
    let evs = store
        .stream_filtered_chunked(store.node_id().stream(0.into()), 0..=10000, AllQuery)
        .take_until_signaled(tokio::time::sleep(Duration::from_secs(2)))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?
        .into_iter()
        .flat_map(|c| c.data);
    assert_eq!(evs.count(), 1000);
    // Make sure the root didn't change
    let root_after_query = stream.published_tree().unwrap().root();
    assert_eq!(root_after_append, root_after_query);

    // Wait here for compaction (or make it configurable in [`SwarmConfig`] ..)
    tokio::time::sleep(Duration::from_secs(60)).await;

    let root_after_compaction = stream.published_tree().unwrap().root();
    assert!(root_after_append != root_after_compaction);
    Ok(())
}
