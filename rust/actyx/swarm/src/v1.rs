use crate::SwarmOffsets;
use crate::{event_store, selection::StreamEventSelection};
use crate::{AxTreeExt, BanyanStore, TT};
use actyxos_sdk::{
    service::OffsetsResponse, Event, EventKey, LamportTimestamp, Metadata, Offset, OffsetOrMin, Payload, StreamId,
    StreamNr, TagSet, Timestamp,
};
use anyhow::Result;
use banyan::forest;
use fnv::FnvHashSet;
use forest::FilteredChunk;
use futures::stream::BoxStream;
use futures::{future::BoxFuture, prelude::*};
use std::{
    cmp::Reverse,
    convert::{TryFrom, TryInto},
    num::NonZeroU64,
    ops::RangeInclusive,
};
use trees::axtrees::{AxKey, TagsQuery};

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
        let stream = self.get_or_create_own_stream(stream_nr);
        let n = events.len();
        let guard = stream.lock().await;
        let mut store = self.lock();
        let mut lamports = store.reserve_lamports(events.len())?.peekable();
        let min_lamport = *lamports.peek().unwrap();
        let kvs = lamports
            .zip(events)
            .map(|(lamport, (tags, payload))| (AxKey::new(tags, lamport, timestamp), payload));
        tracing::debug!("publishing {} events on stream {}", n, stream_nr);
        let mut min_offset = OffsetOrMin::MIN;
        self.transform_stream(&guard, |txn, tree| {
            min_offset = min_offset.max(tree.offset());
            txn.extend_unpacked(tree, kvs)
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

impl event_store::EventStore for BanyanStore {
    fn is_local(&self, stream_id: StreamId) -> bool {
        self.is_local(stream_id)
    }

    fn has_stream(&self, stream_id: StreamId) -> bool {
        self.has_stream(stream_id)
    }

    fn known_streams(&self) -> BoxStream<'static, StreamId> {
        let mut seen = FnvHashSet::default();
        self.stream_known_streams()
            .filter_map(move |stream_id| future::ready(if seen.insert(stream_id) { Some(stream_id) } else { None }))
            .boxed()
    }

    fn offsets(&self) -> stream::BoxStream<'static, OffsetsResponse> {
        #[allow(clippy::redundant_closure)]
        self.data.offsets.new_projection(|x| OffsetsResponse::from(x)).boxed()
    }

    fn persist(
        &self,
        events: Vec<(TagSet, Payload)>,
    ) -> BoxFuture<'static, anyhow::Result<Vec<event_store::PersistenceMeta>>> {
        self.clone().persist0(events).boxed()
    }

    fn forward_stream(&self, stream_selection: StreamEventSelection) -> event_store::EventStreamOrError {
        let stream_id = stream_selection.stream_id;
        debug_assert!(self.has_stream(stream_id));
        debug_assert!(stream_selection.from_exclusive < stream_selection.to_inclusive);
        let (trees, forest) = self.tree_stream(stream_id);
        let range = get_range_inclusive(&stream_selection);
        let query = TagsQuery::new(stream_selection.tag_subscriptions);
        future::ok(
            forest
                .stream_trees_chunked(query, trees, range, &|_| ())
                .map_ok(move |chunk| stream::iter(events_from_chunk(stream_id, chunk)))
                .take_while(|x| future::ready(x.is_ok()))
                .filter_map(|x| future::ready(x.ok()))
                .flatten()
                .boxed(),
        )
        .boxed()
    }

    fn backward_stream(&self, stream_selection: StreamEventSelection) -> event_store::EventStreamReverseOrError {
        let stream_id = stream_selection.stream_id;
        debug_assert!(stream_selection.from_exclusive < stream_selection.to_inclusive);
        debug_assert!(self.has_stream(stream_id));
        let (trees, forest) = self.tree_stream(stream_id);
        let range = get_range_inclusive(&stream_selection);
        let query = TagsQuery::new(stream_selection.tag_subscriptions);
        future::ok(
            forest
                .stream_trees_chunked_reverse(query, trees, range, &|_| ())
                .map_ok(move |chunk| stream::iter(events_from_chunk_rev(stream_id, chunk)))
                .take_while(|x| future::ready(x.is_ok()))
                .filter_map(|x| future::ready(x.ok()))
                .flatten()
                .boxed(),
        )
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

/// Take a block of banyan events and convert them into events.
fn events_from_chunk(stream_id: StreamId, chunk: FilteredChunk<TT, Payload, ()>) -> Vec<Event<Payload>> {
    chunk
        .data
        .into_iter()
        .map(move |(offset, key, payload)| to_ev(offset, key, stream_id, payload))
        .collect()
}

/// Take a block of banyan events and convert them into events, reversing them.
fn events_from_chunk_rev(stream_id: StreamId, chunk: FilteredChunk<TT, Payload, ()>) -> Vec<Reverse<Event<Payload>>> {
    chunk
        .data
        .into_iter()
        .rev()
        .map(move |(offset, key, payload)| Reverse(to_ev(offset, key, stream_id, payload)))
        .collect()
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::{event_store::EventStore, SwarmConfig};
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
