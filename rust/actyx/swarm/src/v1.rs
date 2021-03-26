use crate::access::{
    common::StreamEventSelection, ConsumerAccessError, EventOrHeartbeat, EventOrHeartbeatStreamOrError,
    EventStoreConsumerAccess, EventStreamOrError,
};
use crate::{BanyanStore, TT};
use actyxos_sdk::{
    service::snapshots::{
        InvalidateSnapshotsRequest, RetrieveSnapshotRequest, RetrieveSnapshotResponse, StoreSnapshotRequest,
    },
    Event, EventKey, LamportTimestamp, Metadata, NodeId, Offset, OffsetOrMin, Payload, StreamId, StreamNr, TagSet,
    Timestamp,
};
use anyhow::Result;
use banyan::{
    forest::{self},
    index::IndexRef,
};
use fnv::FnvHashSet;
use forest::FilteredChunk;
use futures::future::BoxFuture;
use futures::prelude::*;
use futures::stream::BoxStream;
use std::{
    collections::BTreeSet,
    convert::{TryFrom, TryInto},
    ops::RangeInclusive,
    time::Duration,
};
use trees::{
    axtrees::{AxKey, TagsQuery},
    OffsetMapOrMax, PublishSnapshot, RootMap, StreamHeartBeat,
};

fn get_range_inclusive(selection: &StreamEventSelection) -> RangeInclusive<u64> {
    let min = (selection.from_exclusive - OffsetOrMin::MIN) as u64;
    let max = (selection.to_inclusive - OffsetOrMin::ZERO) as u64;
    min..=max
}

impl BanyanStore {
    /// Tell the store that we have seen an unvalidated root map
    pub(crate) fn received_root_map(
        &self,
        _node_id: NodeId,
        _lamport: LamportTimestamp,
        root_map: RootMap,
    ) -> impl Future<Output = ()> {
        for (stream_id, entry) in root_map.0 {
            if let Ok(root) = entry.cid.try_into() {
                self.update_root(stream_id, root);
            } else {
                tracing::warn!("Cid that is not SHA2-256")
            }
        }
        future::ready(())
    }

    pub(crate) async fn v1_gossip_publish(self, topic: String) {
        ax_futures_util::stream::interval(Duration::from_secs(10))
            .for_each(move |_| self.publish_root_map(&topic))
            .await
    }

    /// Start V1 gossip ingest. This reads heartbeats from a gossipsub topic and ingests them.
    ///
    /// This should be launched only once, and the join handle should be stored.
    pub(crate) async fn v1_gossip_ingest(self, topic: String) {
        let store = self.clone();
        self.0
            .ipfs
            .subscribe(&topic)
            .unwrap()
            .filter_map(|msg| future::ready(serde_cbor::from_slice::<PublishSnapshot>(msg.as_slice()).ok()))
            .for_each(move |heartbeat| {
                tracing::debug!("{} received heartbeat", self.ipfs().local_node_name());
                store.received_root_map(heartbeat.node, heartbeat.lamport, heartbeat.roots)
            })
            .await
    }

    pub(crate) fn publish_root_map(&self, topic: &str) -> impl Future<Output = ()> {
        let node = self.node_id();
        let lamport = LamportTimestamp::from(self.0.index_store.lock().lamport());
        let roots = self.0.maps.lock().root_map(node);
        let timestamp = Timestamp::now();
        let msg = PublishSnapshot {
            node,
            lamport,
            timestamp,
            roots,
        };
        let blob = serde_cbor::to_vec(&msg).unwrap();
        let _ = self.0.ipfs.publish(topic, blob);
        future::ready(())
    }

    async fn persist0(self, events: Vec<(TagSet, Payload)>) -> Result<Vec<(LamportTimestamp, Offset, StreamNr)>> {
        let n = events.len() as u32;
        let last_lamport = self.0.index_store.lock().increase_lamport(n)?;
        let min_lamport = last_lamport - (n as u64) + 1;
        let stream_nr = StreamNr::from(0); // TODO
        let timestamp = Timestamp::now();
        let kvs = events
            .into_iter()
            .enumerate()
            .map(move |(i, (tags, payload))| {
                let key = AxKey::new(tags, min_lamport + (i as u64), timestamp);
                (key, payload)
            })
            .collect::<Vec<_>>();
        tracing::debug!("publishing {} events on stream {}", kvs.len(), stream_nr);
        let mut min_offset = 0u32;
        let _ = self
            .transform_stream(stream_nr, |txn, tree| {
                min_offset = tree.count() as u32;
                txn.extend_unpacked(tree, kvs)
            })
            .await?;

        let keys = (0..n)
            .map(|i| {
                let lamport = (min_lamport + (i as u64)).into();
                let offset = (min_offset + (i as u32)).into();
                (lamport, offset, stream_nr)
            })
            .collect();
        Ok(keys)
    }

    pub(crate) fn update_present(&self, stream_id: StreamId, offset: u64) -> anyhow::Result<()> {
        let offset = actyxos_sdk::OffsetOrMin::from(i64::try_from(offset)?);
        self.0.present.transform(|present| {
            let mut present = present.clone();
            present.update(stream_id, offset);
            Ok(Some(present))
        })
    }

    pub(crate) fn update_highest_seen(&self, stream_id: StreamId, offset: u64) -> anyhow::Result<()> {
        let offset = actyxos_sdk::OffsetOrMin::from(i64::try_from(offset)?);
        self.0.highest_seen.transform(|highest_seen| {
            Ok(if highest_seen.offset(stream_id) < offset {
                let mut highest_seen = highest_seen.clone();
                highest_seen.update(stream_id, offset);
                Some(highest_seen)
            } else {
                None
            })
        })
    }
}

pub trait EventStore: Clone + Send + Unpin + Sync + 'static {
    /// Persist events and assign offsets and lamports
    fn persist<'a>(
        &self,
        events: Vec<(TagSet, Payload)>,
    ) -> BoxFuture<'a, Result<Vec<(LamportTimestamp, Offset, StreamNr)>>>;
}

impl EventStore for BanyanStore {
    fn persist<'a>(
        &self,
        events: Vec<(TagSet, Payload)>,
    ) -> BoxFuture<'a, Result<Vec<(LamportTimestamp, Offset, StreamNr)>>> {
        self.clone().persist0(events).boxed()
    }
}

impl EventStoreConsumerAccess for BanyanStore {
    fn local_stream_ids(&self) -> BTreeSet<StreamId> {
        let state = self.0.maps.lock();

        state.own_streams.keys().map(|x| self.0.node_id.stream(*x)).collect()
    }

    fn stream_forward(&self, events: StreamEventSelection, must_exist: bool) -> EventOrHeartbeatStreamOrError {
        let stream_id = events.stream_id;
        if must_exist && !self.has_stream(stream_id) {
            return future::err(ConsumerAccessError::UnknownStream(stream_id)).boxed();
        }
        let (trees, forest) = self.tree_stream(stream_id);
        let range = get_range_inclusive(&events);
        let query = TagsQuery::new(events.subscription_set);
        // stream the events in ascending order from the trees
        let events_and_heartbeats_from_trees = forest
            .stream_trees_chunked(query, trees, range, &last_lamport_from_index_ref)
            .map_ok(move |chunk| stream::iter(events_or_heartbeat_from_chunk(stream_id, chunk)))
            .take_while(|x| future::ready(x.is_ok()))
            .filter_map(|x| future::ready(x.ok()))
            .flatten();
        // mix in heartbeats from latest so we can make progress even if we don't get events
        let heartbeats_from_latest = self.latest_stream(stream_id).map(move |(lamport, offset)| {
            EventOrHeartbeat::Heartbeat(StreamHeartBeat::new(stream_id, lamport, offset))
        });
        future::ok(stream::select(events_and_heartbeats_from_trees, heartbeats_from_latest).boxed()).boxed()
    }

    fn stream_backward(&self, events: StreamEventSelection) -> EventStreamOrError {
        let stream_id = events.stream_id;
        let (trees, forest) = self.tree_stream(stream_id);
        let range = get_range_inclusive(&events);
        let query = TagsQuery::new(events.subscription_set);
        future::ok(
            forest
                .stream_trees_chunked_reverse(query, trees, range, &|_| {})
                .map_ok(move |chunk| stream::iter(events_from_chunk_rev(stream_id, chunk)))
                .take_while(|x| future::ready(x.is_ok()))
                .filter_map(|x| future::ready(x.ok()))
                .flatten()
                .boxed(),
        )
        .boxed()
    }

    fn stream_last_seen(&self, stream_id: StreamId) -> stream::BoxStream<'static, StreamHeartBeat> {
        let stream = self.get_or_create_replicated_stream(stream_id);
        stream
            .latest_seen
            .new_observer()
            .filter_map(|x| async move { x })
            .map(move |(lamport, offset)| StreamHeartBeat::new(stream_id, lamport, offset))
            .boxed()
    }

    fn stream_known_streams(&self) -> stream::BoxStream<'static, StreamId> {
        let mut seen = FnvHashSet::default();
        self.stream_known_streams()
            .filter_map(move |stream_id| future::ready(if seen.insert(stream_id) { Some(stream_id) } else { None }))
            .boxed()
    }
}

fn to_ev(offset: u64, key: AxKey, stream: StreamId, payload: Payload) -> Event<Payload> {
    Event {
        payload,
        key: EventKey {
            lamport: key.lamport(),
            offset: offset.try_into().expect("TODO"),
            stream,
        },
        meta: Metadata {
            timestamp: key.time(),
            tags: key.into_tags(),
        },
    }
}

/// Given an ax index ref, extract the last lamport timestamp
fn last_lamport_from_index_ref(r: IndexRef<TT>) -> LamportTimestamp {
    match r {
        IndexRef::Branch(branch) => branch.summaries.lamport_range().max,
        IndexRef::Leaf(leaf) => leaf.keys.lamport_range().max,
    }
}

/// Take a block of banyan events and convert them into events
/// wrapped in EventOrHeartbeat.
///
/// In case the last event was filtered out, a placeholder heartbeat is added.
fn events_or_heartbeat_from_chunk(
    stream_id: StreamId,
    chunk: FilteredChunk<TT, Payload, LamportTimestamp>,
) -> Vec<EventOrHeartbeat> {
    let last_offset = chunk.range.end - 1;
    let has_last = chunk
        .data
        .last()
        .map(|(offset, _, _)| *offset == last_offset)
        .unwrap_or_default();
    let last_lamport = chunk.extra;
    let mut result = chunk
        .data
        .into_iter()
        .map(move |(offset, key, payload)| EventOrHeartbeat::Event(to_ev(offset, key, stream_id, payload)))
        .collect::<Vec<EventOrHeartbeat>>();
    if !has_last {
        result.push(EventOrHeartbeat::Heartbeat(StreamHeartBeat::new(
            stream_id,
            last_lamport,
            last_offset.try_into().unwrap(),
        )))
    }
    result
}

/// Take a block of banyan events and convert them into ActyxOS Event<Payload> events, reversing them
fn events_from_chunk_rev(stream_id: StreamId, chunk: FilteredChunk<TT, Payload, ()>) -> Vec<Event<Payload>> {
    chunk
        .data
        .into_iter()
        .rev()
        .map(move |(offset, key, payload)| to_ev(offset, key, stream_id, payload))
        .collect()
}

/// Provides the current highest validated offsets as a sampled stream
/// without back pressure, where the latest element is always available.
pub trait Present: Clone + Send + Unpin + Sync + 'static {
    fn stream(&self) -> BoxStream<'static, OffsetMapOrMax>;
}

impl Present for BanyanStore {
    fn stream(&self) -> stream::BoxStream<'static, OffsetMapOrMax> {
        self.0.present.new_observer().boxed()
    }
}

/// Provides the highest seen, but not necessarily validated RootMap as a
/// sampled stream without back pressure, where the latest element is always
/// available.
pub trait HighestSeen: Clone + Send + Unpin + Sync + 'static {
    fn stream(&self) -> BoxStream<'static, OffsetMapOrMax>;
}

impl HighestSeen for BanyanStore {
    fn stream(&self) -> stream::BoxStream<'static, OffsetMapOrMax> {
        self.0.highest_seen.new_observer().boxed()
    }
}

pub trait SnapshotStore: Clone + Send + Unpin + Sync + 'static {
    fn store_snapshot<'a>(&self, req: StoreSnapshotRequest) -> BoxFuture<'a, Result<bool>>;

    fn invalidate_snapshots<'a>(&self, req: InvalidateSnapshotsRequest) -> BoxFuture<'a, Result<()>>;

    fn retrieve_snapshot<'a>(
        &self,
        req: RetrieveSnapshotRequest,
    ) -> BoxFuture<'a, Result<Option<RetrieveSnapshotResponse>>>;

    fn invalidate_all_snapshots<'a>(&self) -> BoxFuture<'a, Result<()>>;
}

impl SnapshotStore for BanyanStore {
    fn store_snapshot<'a>(&self, req: StoreSnapshotRequest) -> BoxFuture<'a, Result<bool>> {
        future::ready(self.0.index_store.lock().store_snapshot(req)).boxed()
    }

    fn invalidate_snapshots<'a>(&self, req: InvalidateSnapshotsRequest) -> BoxFuture<'a, Result<()>> {
        future::ready(self.0.index_store.lock().invalidate_snapshots(req)).boxed()
    }

    fn retrieve_snapshot<'a>(
        &self,
        req: RetrieveSnapshotRequest,
    ) -> BoxFuture<'a, Result<Option<RetrieveSnapshotResponse>>> {
        future::ready(self.0.index_store.lock().retrieve_snapshot(req)).boxed()
    }

    fn invalidate_all_snapshots<'a>(&self) -> BoxFuture<'a, Result<()>> {
        future::ready(self.0.index_store.lock().invalidate_all_snapshots()).boxed()
    }
}
