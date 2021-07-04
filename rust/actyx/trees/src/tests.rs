use crate::{
    axtrees::{AxKey, AxTrees, Sha256Digest},
    query::{LamportQuery, TagExprQuery, TimeQuery},
    stags,
    tags::ScopedTagSet,
    AxTree,
};
use actyx_sdk::{tag, tags, LamportTimestamp, Payload, TagSet, Timestamp};
use banyan::{
    query::{AllQuery, OffsetRangeQuery, Query},
    store::{BranchCache, MemStore},
    Forest, StreamBuilder, Transaction,
};
use futures::prelude::*;
use parking_lot::Mutex;
use serde_json::json;
use std::sync::Arc;

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

    fn time(&self) -> Timestamp {
        Timestamp::new(*self.time.lock())
    }

    fn increase_time(&self, delta: u64) {
        *self.time.lock() += delta;
    }

    fn generate_json(
        &self,
        mut tag_generator: impl FnMut(usize) -> TagSet + 'static,
        mut payload_generator: impl FnMut(usize) -> serde_json::Value + 'static,
    ) -> Box<dyn Iterator<Item = (AxKey, Payload)>> {
        let this = self.clone();
        Box::new((0..).map(move |index| {
            let tags = tag_generator(index).into();
            let payload = Payload::from_json_value(payload_generator(index)).unwrap();
            let key = AxKey::new(tags, this.next_lamport(), this.time());
            (key, payload)
        }))
    }

    /// an infinite stream of events for a counter that cycles from 0 to cycle - 1
    pub fn generate_counter(&self, tags: TagSet, cycle: u64) -> Box<dyn Iterator<Item = (AxKey, Payload)>> {
        let mut counter = 0;
        self.generate_json(
            move |_| tags.clone(),
            move |_| {
                counter = (counter + 1) % cycle;
                json!({ "counter": counter })
            },
        )
    }

    /// an infinite stream of events for a switch that turns on and off
    pub fn generate_switch(&self, tags: TagSet) -> Box<dyn Iterator<Item = (AxKey, Payload)>> {
        let mut on = false;
        self.generate_json(
            move |_| tags.clone(),
            move |_| {
                on = !on;
                json!({ "on": on })
            },
        )
    }

    /// an infinite stream of "article production" events
    ///
    /// there will be a new, distinct tag for every second event.
    pub fn generate_article(&self, base_tags: TagSet) -> Box<dyn Iterator<Item = (AxKey, Payload)>> {
        let mut producing = false;
        self.generate_json(
            move |index| {
                let id = index / 2;
                let mut tags = base_tags.clone();
                tags += tag!("article_id:") + id.to_string();
                tags
            },
            move |index| {
                let id = index / 2;
                producing = !producing;
                if producing {
                    json!({ "type": "start", "article_id": id })
                } else {
                    json!({ "type": "stop", "article_id": id })
                }
            },
        )
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

type AxTxn = Transaction<AxTrees, MemStore<Sha256Digest>, MemStore<Sha256Digest>>;

fn test_txn() -> AxTxn {
    let store = MemStore::new(usize::max_value(), Sha256Digest::new);
    let branch_cache = BranchCache::new(1000);
    AxTxn::new(Forest::new(store.clone(), branch_cache), store)
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

fn generate_random_events(n: usize) -> Vec<(AxKey, Payload)> {
    let gen = Generator::new();
    let events = gen
        .combine(vec![
            gen.generate_counter(tags!["machine", "machine:1", "article", "article:toothpaste"], 15),
            gen.generate_counter(tags!["machine", "machine:2", "article", "article:warhead"], 100),
            gen.generate_article(tags!["location", "location:hall1", "circuit", "circuit:lights"]),
            gen.generate_article(tags!["location", "location:hall1", "circuit", "circuit:heating"]),
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

/// Filter a single tree by a query
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

/// Filter a stream of trees by a query
async fn filter_tree_streamed(
    txn: &AxTxn,
    tree: &AxTree,
    query: impl Query<AxTrees> + Clone + 'static,
) -> anyhow::Result<Vec<(u64, AxKey, Payload)>> {
    let trees = stream::iter(txn.left_roots(tree)?);
    txn.stream_trees(query, trees)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()
}

/// brute force check if a key matches a dnf query consisting of n tag sets
fn matches(key: &AxKey, dnf: &[ScopedTagSet]) -> bool {
    dnf == vec![ScopedTagSet::empty()] || dnf.iter().any(|set| set.is_subset(key.tags()))
}

/// Roundtrip test from events to banyan tree and back, for the given events
fn events_banyan_tree_roundtrip_with(events: Vec<(AxKey, Payload)>) -> anyhow::Result<()> {
    // create a transaction so we can write banyan trees
    let txn = test_txn();
    // create a tree
    let mut builder = StreamBuilder::debug();
    txn.extend(&mut builder, events.clone())?;
    // get back the events
    let events1 = txn
        .collect(&builder.snapshot())?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    // check that they are the same
    assert_eq!(events, events1);
    Ok(())
}

/// Roundtrip test from events to banyan tree and back
#[test]
fn events_banyan_tree_roundtrip() -> anyhow::Result<()> {
    for events in vec![generate_events(10000), generate_random_events(10000)] {
        events_banyan_tree_roundtrip_with(events)?;
    }
    Ok(())
}

/// Roundtrip test from events to banyan tree and back, query with no limitiations, with the given events
async fn events_banyan_tree_simple_queries_with(events: Vec<(AxKey, Payload)>) -> anyhow::Result<()> {
    // add the offsets
    let events_with_offset = || add_offsets(events.clone());
    // create a transaction so we can write banyan trees
    let txn = test_txn();
    // create a tree
    // this is pretty expensive, since the tree is configured to have a complex structure.
    let mut builder = StreamBuilder::debug();
    txn.extend(&mut builder, events.clone())?;
    let tree = builder.snapshot();
    // try all query
    {
        let query = AllQuery;
        let events0 = events_with_offset().collect::<Vec<_>>();
        let events1 = filter_tree(&txn, &tree, query).await?;
        assert_eq!(events0, events1);
    }
    // try offset query, different ranges
    for offset_range in vec![100..6666, 0..1] {
        let offset_range1 = offset_range.clone();
        let events0 = events_with_offset()
            .filter(move |(offset, _, _)| offset_range1.contains(offset))
            .collect::<Vec<_>>();
        let query = OffsetRangeQuery::from(offset_range);

        let events1 = filter_tree(&txn, &tree, query.clone()).await?;
        assert_eq!(events0, events1);

        let events1 = filter_tree_streamed(&txn, &tree, query).await?;
        assert_eq!(events0, events1);
    }
    // try lamport query, different ranges
    for lamport_range in vec![LamportTimestamp::new(1)..LamportTimestamp::new(10000)] {
        let lamport_range1 = lamport_range.clone();
        let events0 = events_with_offset()
            .filter(move |(_, key, _)| lamport_range1.contains(&key.lamport()))
            .collect::<Vec<_>>();
        let query = LamportQuery::from(lamport_range);
        let events1 = filter_tree(&txn, &tree, query.clone()).await?;
        assert_eq!(events0, events1);

        let events1 = filter_tree_streamed(&txn, &tree, query).await?;
        assert_eq!(events0, events1);
    }
    // try time query, different ranges
    for time_range in vec![Timestamp::new(AX_EPOCH + 1_000_000)..Timestamp::new(AX_EPOCH + 666_000_000)] {
        let time_range1 = time_range.clone();
        let events0 = events_with_offset()
            .filter(move |(_, key, _)| time_range1.contains(&key.time()))
            .collect::<Vec<_>>();
        let query = TimeQuery::from(time_range);
        let events1 = filter_tree(&txn, &tree, query.clone()).await?;
        assert_eq!(events0, events1);

        let events1 = filter_tree_streamed(&txn, &tree, query).await?;
        assert_eq!(events0, events1);
    }
    // try tag query, different dnfs
    for tags in vec![
        vec![stags! {"machine:1"}],
        vec![stags! {"unknown tag"}],
        vec![stags! {"unknown tag"}, stags! {"machine:1"}],
        vec![stags! {"unknown tag", "machine:1"}],
        vec![stags! {}],
        vec![],
        vec![stags! {"location:hall1", "whatever", "circuit:lights"}],
        vec![stags! {"location:hall1", "circuit:lights"}],
        vec![stags! {"location:hall1"}, stags! {"machine:1"}],
    ] {
        // compute the reference
        let tags1 = tags.clone();
        let events0 = events_with_offset()
            .filter(move |(_, key, _)| matches(key, &tags1))
            .collect::<Vec<_>>();
        // get back the events, filtered by tags
        let query = TagExprQuery::new(tags.clone(), LamportQuery::all(), TimeQuery::all());
        let events1 = filter_tree(&txn, &tree, query.clone()).await?;
        assert_eq!(events0, events1);

        let events1 = filter_tree_streamed(&txn, &tree, query).await?;
        assert_eq!(events0, events1);
    }
    Ok(())
}

#[tokio::test]
async fn events_banyan_tree_simple_queries() -> anyhow::Result<()> {
    for events in vec![generate_events(10000), generate_random_events(10000)] {
        events_banyan_tree_simple_queries_with(events).await?;
    }
    Ok(())
}
