use actyxos_sdk::{tags, LamportTimestamp, Payload, TagSet, TimeStamp};
use banyan::{
    forest::{BranchCache, Config, CryptoConfig, Forest, Transaction},
    memstore::MemStore,
    query::{AllQuery, Query},
    tree::Tree,
};
use futures::prelude::*;
use parking_lot::Mutex;
use serde_json::json;
use std::sync::Arc;
use trees::axtrees::{AxKey, AxTree, AxTrees, Sha256Digest, TagsQuery};

const AX_EPOCH: u64 = 1451606400000000;

#[derive(Clone)]
struct Generator {
    lamport: Arc<Mutex<u64>>,
    time: Arc<Mutex<u64>>,
}

impl Default for Generator {
    fn default() -> Self {
        Self {
            lamport: Default::default(),
            time: Arc::new(Mutex::new(AX_EPOCH)),
        }
    }
}

impl Generator {
    fn new() -> Self {
        Self {
            lamport: Default::default(),
            time: Default::default(),
        }
    }

    fn next_lamport(&self) -> LamportTimestamp {
        let mut lamport = self.lamport.lock();
        let result = *lamport;
        *lamport += 1;
        LamportTimestamp::from(result)
    }

    fn time(&self) -> TimeStamp {
        TimeStamp::new(*self.time.lock())
    }

    fn increase_time(&self, delta: u64) {
        *self.time.lock() += delta;
    }

    fn generate_json(
        &self,
        tags: TagSet,
        mut payload_generator: impl FnMut(usize) -> serde_json::Value + 'static,
    ) -> Box<dyn Iterator<Item = (AxKey, Payload)>> {
        let this = self.clone();
        Box::new((0..).map(move |index| {
            let payload = Payload::from_json_value(payload_generator(index)).unwrap();
            let key = AxKey::new(tags.clone(), this.next_lamport(), this.time());
            (key, payload)
        }))
    }

    /// an infinite stream of events for a counter that cycles from 0 to cycle - 1
    pub fn generate_counter(&self, tags: TagSet, cycle: u64) -> Box<dyn Iterator<Item = (AxKey, Payload)>> {
        let mut counter = 0;
        self.generate_json(tags, move |_| {
            counter = (counter + 1) % cycle;
            json!({ "counter": counter })
        })
    }

    /// an infinite stream of events for a switch that turns on and off
    pub fn generate_switch(&self, tags: TagSet) -> Box<dyn Iterator<Item = (AxKey, Payload)>> {
        let mut on = false;
        self.generate_json(tags, move |_| {
            on = !on;
            json!({ "on": on })
        })
    }

    /// combine multiple streams of events
    pub fn combine(
        &self,
        mut iters: Vec<Box<dyn Iterator<Item = (AxKey, Payload)>>>,
    ) -> Box<dyn Iterator<Item = (AxKey, Payload)>> {
        let this = self.clone();
        Box::new((0..).flat_map(move |_| {
            let elements = iters.iter_mut().filter_map(|x| x.next()).collect::<Vec<_>>();
            this.increase_time(1000000);
            elements
        }))
    }
}

type AxTxn = Transaction<AxTrees, Payload, MemStore<Sha256Digest>, MemStore<Sha256Digest>>;

fn test_txn() -> AxTxn {
    let store = MemStore::new(usize::max_value(), Sha256Digest::new);
    let branch_cache = BranchCache::new(1000);
    let config = Config::debug();
    let crypto_config = CryptoConfig::default();
    AxTxn::new(Forest::new(store.clone(), branch_cache, crypto_config, config), store)
}

fn generate_events(n: usize) -> Vec<(AxKey, Payload)> {
    let gen = Generator::new();
    let events = gen
        .combine(vec![
            gen.generate_counter(tags!["machine", "machine:1", "article", "article:toothpaste"], 15),
            gen.generate_counter(tags!["machine", "machine:2", "article", "article:warhead"], 100),
            gen.generate_switch(tags!["location", "location:hall1", "circuit", "circuit:lights"]),
            gen.generate_switch(tags!["location", "location:hall1", "circuit", "circuit:heating"]),
        ])
        .take(n)
        .collect::<Vec<_>>();
    events
}

fn add_offsets(events: impl IntoIterator<Item = (AxKey, Payload)>) -> impl Iterator<Item = (u64, AxKey, Payload)> {
    events
        .into_iter()
        .enumerate()
        .map(|(offset, (key, value))| (offset as u64, key, value))
}

async fn filter_tree(
    txn: &AxTxn,
    tree: &AxTree,
    query: impl Query<AxTrees> + Clone + 'static,
) -> anyhow::Result<Vec<(u64, AxKey, Payload)>> {
    txn.stream_filtered(&tree, query)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()
}

/// brute force check if a key matches a dnf query consisting of n tag sets
fn matches(key: &AxKey, dnf: &[TagSet]) -> bool {
    dnf.iter().any(|set| set.is_subset(key.tags()))
}

/// Roundtrip test from events to banyan tree and back, query with no limitiations
#[tokio::test]
async fn events_banyan_tree_subscription_set() -> anyhow::Result<()> {
    // create some events, with tags and all
    let events = generate_events(10000);
    // add the offsets
    let events_with_offset = || add_offsets(events.clone());
    // create a transaction so we can write banyan trees
    let txn = test_txn();
    // create a tree
    let tree = txn.extend(&Tree::default(), events.clone())?;
    // try all query
    {
        let query = AllQuery;
        let events0 = events_with_offset().collect::<Vec<_>>();
        let events1 = filter_tree(&txn, &tree, query).await?;
        assert_eq!(events0, events1);
    }
    // try subscription set queries
    for sub in vec![vec![], vec![TagSet::empty()]] {
        let query = TagsQuery::new(sub);
        let events1 = filter_tree(&txn, &tree, query.clone()).await?;
        let events0 = events_with_offset()
            .filter(move |(_, key, _)| matches(key, query.tags()))
            .collect::<Vec<_>>();
        assert_eq!(events0, events1);
    }
    Ok(())
}
