use std::{collections::BTreeMap, convert::TryFrom, time::Duration};

use crate::{BanyanStore, Config, DbPath, Ipfs, SqliteIndexStore};
use actyxos_sdk::{NodeId, Payload, StreamNr, Tag, TagSet};
use ax_futures_util::stream::interval;
use banyan::query::AllQuery;
use futures::prelude::*;
use ipfs_embed::Config as IpfsConfig;
use libipld::Cid;

struct Tagger(BTreeMap<&'static str, Tag>);

impl Tagger {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn tag(&mut self, name: &'static str) -> Tag {
        self.0
            .entry(name)
            .or_insert_with(|| Tag::new(name.into()).unwrap())
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

fn setup_logger() {
    tracing_log::LogTracer::init().ok();
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();
}

#[tokio::test]
#[ignore]
async fn smoke() -> anyhow::Result<()> {
    setup_logger();
    let mut tagger = Tagger::new();
    let mut ev = move |tag| (tagger.tags(&[tag]), Payload::empty());
    let ipfs = Ipfs::new(IpfsConfig::new(None, 0)).await?;
    let config = Config::test();
    let store = BanyanStore::new(ipfs.clone(), config, SqliteIndexStore::open(DbPath::Memory)?)?;
    tokio::task::spawn(store.stream_filtered_stream_ordered(AllQuery).for_each(|x| {
        tracing::info!("got event {:?}", x);
        future::ready(())
    }));
    let stream_nr = StreamNr::try_from(1)?;
    tracing::info!("append first event!");
    let _ = store.append(stream_nr, vec![ev("a")]).await?.unwrap();
    tracing::info!("append second event!");
    let root = store.append(stream_nr, vec![ev("b")]).await?.unwrap();
    tracing::info!("done!");
    let node1 = NodeId::from_bytes(&[1u8; 32])?;
    let stream1 = node1.stream(StreamNr::try_from(1)?);
    tracing::info!("update_root !!!");
    store.update_root(stream1, root);
    tracing::info!("update_root !!!");
    let stream2 = node1.stream(StreamNr::try_from(2)?);
    store.update_root(stream2, root);
    store.compact_once().await?;
    tokio::task::spawn(interval(Duration::from_secs(1)).for_each(move |_| {
        let store = store.clone();
        let mut tagger = Tagger::new();
        let mut ev = move |tag| (tagger.tags(&[tag]), Payload::empty());
        async move {
            let _ = store.append(stream_nr, vec![ev("a")]).await.unwrap();
        }
    }));
    tokio::task::spawn(ipfs.subscribe("test").unwrap().for_each(|msg| {
        tracing::error!("XXXX msg {:?}", msg);
        future::ready(())
    }));
    tokio::time::sleep(Duration::from_secs(1000)).await;
    Ok(())
}
