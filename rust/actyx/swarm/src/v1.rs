use crate::access::{
    common::StreamEventSelection, ConsumerAccessError, EventOrHeartbeat, EventOrHeartbeatStreamOrError,
    EventStoreConsumerAccess, EventStreamOrError,
};
use crate::SwarmOffsets;
use crate::{AxTreeExt, BanyanStore, TT};
use actyxos_sdk::{
    service::OffsetsResponse, Event, EventKey, LamportTimestamp, Metadata, Offset, OffsetOrMin, Payload, StreamId,
    StreamNr, TagSet, Timestamp,
};
use anyhow::Result;
use ax_futures_util::prelude::AxStreamExt;
use banyan::{index::IndexRef, FilteredChunk};
use fnv::FnvHashSet;
use futures::stream::BoxStream;
use futures::{channel::mpsc, future::BoxFuture, prelude::*};
use std::{
    collections::BTreeSet,
    convert::{TryFrom, TryInto},
    num::NonZeroU64,
    ops::RangeInclusive,
};
use trees::{
    axtrees::{AxKey, TagsQuery},
    StreamHeartBeat,
};

fn get_range_inclusive(selection: &StreamEventSelection) -> RangeInclusive<u64> {
    let min = (selection.from_exclusive - OffsetOrMin::MIN) as u64;
    let max = (selection.to_inclusive - OffsetOrMin::ZERO) as u64;
    min..=max
}

pub type PersistenceMeta = (LamportTimestamp, Offset, StreamNr, Timestamp);

impl BanyanStore {
    async fn persist0(self, events: Vec<(TagSet, Payload)>) -> Result<Vec<PersistenceMeta>> {
        let stream_nr = StreamNr::from(0); // TODO
        let timestamp = Timestamp::now();
        let stream = self.get_or_create_own_stream(stream_nr)?;
        let n = events.len();
        let mut guard = stream.lock().await;
        let mut store = self.lock();
        let mut lamports = store.reserve_lamports(events.len())?.peekable();
        let min_lamport = *lamports.peek().unwrap();
        let kvs = lamports
            .zip(events)
            .map(|(lamport, (tags, payload))| (AxKey::new(tags, lamport, timestamp), payload));
        tracing::debug!("publishing {} events on stream {}", n, stream_nr);
        let min_offset = self.transform_stream(&mut guard, |txn, tree| {
            let result = tree.snapshot().offset();
            txn.extend_unpacked(tree, kvs)?;
            Ok(result)
        })?;

        // We start iteration with 0 below, so this is effectively the offset of the first event.
        let starting_offset = Offset::from_offset_or_min(min_offset)
            .map(|x| x.succ())
            .unwrap_or(Offset::ZERO);
        let keys = (0..n)
            .map(|i| {
                let i = i as u64;
                let lamport = min_lamport + i;
                let offset = starting_offset.increase(i).unwrap();
                (lamport, offset, stream_nr, timestamp)
            })
            .collect();
        Ok(keys)
    }

    pub(crate) fn update_present(&self, stream_id: StreamId, offset: OffsetOrMin) {
        if let Some(offset) = Offset::from_offset_or_min(offset) {
            self.data.offsets.transform_mut(|offsets| {
                offsets.present.update(stream_id, offset);
                true
            });
        }
    }

    pub(crate) fn update_highest_seen(&self, stream_id: StreamId, offset: OffsetOrMin) {
        if let Some(offset) = Offset::from_offset_or_min(offset) {
            self.data.offsets.transform_mut(|offsets| {
                if offsets.replication_target.offset(stream_id) < offset {
                    offsets.replication_target.update(stream_id, offset);
                    true
                } else {
                    false
                }
            });
        }
    }
}

pub trait EventStore: Clone + Send + Unpin + Sync + 'static {
    /// Persist events and assign offsets and lamports
    fn persist<'a>(&self, events: Vec<(TagSet, Payload)>) -> BoxFuture<'a, Result<Vec<PersistenceMeta>>>;
}

impl EventStore for BanyanStore {
    fn persist<'a>(&self, events: Vec<(TagSet, Payload)>) -> BoxFuture<'a, Result<Vec<PersistenceMeta>>> {
        self.clone().persist0(events).boxed()
    }
}

impl EventStoreConsumerAccess for BanyanStore {
    fn local_stream_ids(&self) -> BTreeSet<StreamId> {
        self.lock().local_stream_ids()
    }

    fn stream_forward(&self, events: StreamEventSelection, must_exist: bool) -> EventOrHeartbeatStreamOrError {
        let to_inclusive = events.to_inclusive;
        let stream_id = events.stream_id;
        if must_exist && !self.has_stream(stream_id) {
            return future::err(ConsumerAccessError::UnknownStream(stream_id)).boxed();
        }
        let trees = self.tree_stream(stream_id);
        let range = get_range_inclusive(&events);
        let query = TagsQuery::new(events.tag_subscriptions);
        // Used to signal the mixed in `heartbeats_from_latest` stream down
        // below to finish
        let (mut tx, rx) = mpsc::channel(1);

        // stream the events in ascending order from the trees
        let events_and_heartbeats_from_trees = self
            .data
            .forest
            .stream_trees_chunked(query, trees, range, &last_lamport_from_index_ref)
            .take_while(|x| future::ready(x.is_ok()))
            .filter_map(|x| future::ready(x.ok()))
            .take_until_condition(move |chunk| {
                // FIXME: Will this only be triggered once an event outside the queried bounds becomes known?
                let stop_here = Into::<OffsetOrMin>::into(chunk.range.end as u32) > to_inclusive;
                if stop_here {
                    tx.try_send(()).unwrap();
                }
                future::ready(stop_here)
            })
            .map(move |chunk| stream::iter(events_or_heartbeat_from_chunk(stream_id, chunk)))
            .flatten();
        // mix in heartbeats from latest so we can make progress even if we don't get events
        let heartbeats_from_latest = self.latest_stream(stream_id).map(move |(lamport, offset)| {
            EventOrHeartbeat::Heartbeat(StreamHeartBeat::new(stream_id, lamport, offset))
        });
        future::ok(
            stream::select(
                events_and_heartbeats_from_trees,
                heartbeats_from_latest.take_until_signaled(rx.into_future()),
            )
            .boxed(),
        )
        .boxed()
    }

    fn stream_backward(&self, events: StreamEventSelection) -> EventStreamOrError {
        let stream_id = events.stream_id;
        let trees = self.tree_stream(stream_id);
        let range = get_range_inclusive(&events);
        let query = TagsQuery::new(events.tag_subscriptions);
        future::ok(
            self.data
                .forest
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
        let stream = self.latest_stream(stream_id);

        stream
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
    chunk: FilteredChunk<(u64, AxKey, Payload), LamportTimestamp>,
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
fn events_from_chunk_rev(stream_id: StreamId, chunk: FilteredChunk<(u64, AxKey, Payload), ()>) -> Vec<Event<Payload>> {
    chunk
        .data
        .into_iter()
        .rev()
        .map(move |(offset, key, payload)| to_ev(offset, key, stream_id, payload))
        .collect()
}

pub trait Present: Clone + Send + Unpin + Sync + 'static {
    /// Provides both the the currently highest validated offsets (`present`) and the number of
    /// events per Stream which are not yet validated, but were observed within the swarm
    /// (`to_replicate`) as a sampled stream without back pressure, where the latest element is
    /// always available.
    fn offsets(&self) -> BoxStream<'static, OffsetsResponse>;
}

impl From<&SwarmOffsets> for OffsetsResponse {
    fn from(o: &SwarmOffsets) -> Self {
        let to_replicate = o
            .replication_target
            .stream_iter()
            .filter_map(|(stream, target)| {
                let actual = o.present.offset(stream);
                let diff = OffsetOrMin::from(target) - actual;
                u64::try_from(diff).ok().and_then(NonZeroU64::new).map(|o| (stream, o))
            })
            .collect();

        Self {
            present: o.present.clone(),
            to_replicate,
        }
    }
}

impl Present for BanyanStore {
    fn offsets(&self) -> stream::BoxStream<'static, OffsetsResponse> {
        #[allow(clippy::redundant_closure)]
        self.data.offsets.new_projection(|x| OffsetsResponse::from(x)).boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SwarmConfig;
    use ax_futures_util::stream::Drainer;
    use maplit::btreemap;
    use quickcheck::Arbitrary;

    #[tokio::test]
    async fn should_stream_offsets() -> Result<()> {
        let mut cfg = SwarmConfig::test("offset_stream");
        cfg.enable_mdns = false;
        let store = BanyanStore::new(cfg).await?;
        let store_node_id = store.node_id();
        let mut offsets = Drainer::new(store.offsets());

        // Initially only own streams
        let nxt = offsets.next().unwrap().last().cloned().unwrap();
        assert!(nxt.present.streams().all(|x| x.node_id() == store_node_id));
        assert_eq!(nxt.to_replicate, Default::default());

        let mut gen = quickcheck::Gen::new(64);
        let streams: BTreeSet<StreamId> = Arbitrary::arbitrary(&mut gen);

        for (idx, stream) in streams.into_iter().enumerate() {
            let offset = if idx == 0 {
                // Explicitly test the 0 case
                Offset::from(0)
            } else {
                Offset::arbitrary(&mut gen)
            };
            test_offsets(&store, stream, offset);
        }

        Ok(())
    }

    fn test_offsets(store: &BanyanStore, stream: StreamId, offset: Offset) {
        let mut offsets = Drainer::new(store.offsets());

        // Inject root update from `stream`
        store.update_highest_seen(stream, offset.into());

        let nxt = offsets.next().unwrap().last().cloned().unwrap();
        assert!(nxt.present.streams().all(|x| x != stream));
        assert_eq!(
            nxt.to_replicate,
            btreemap! {
                stream => NonZeroU64::new(u64::from(offset) + 1).unwrap()
            }
        );

        // Inject validation of `stream` with `offset - 1`
        if let Some(pred) = offset.pred() {
            store.update_present(stream, pred.into());
            let nxt = offsets.next().unwrap().last().cloned().unwrap();
            assert_eq!(nxt.present.offset(stream), OffsetOrMin::from(pred));
            assert_eq!(
                nxt.to_replicate,
                std::iter::once((stream, NonZeroU64::new(1u64).unwrap())).collect()
            );
        }

        // Inject validation of `stream` with `offset`
        store.update_present(stream, offset.into());
        let nxt = offsets.next().unwrap().last().cloned().unwrap();
        assert_eq!(nxt.present.offset(stream), OffsetOrMin::from(offset));
        assert_eq!(nxt.to_replicate, Default::default());
    }
}
