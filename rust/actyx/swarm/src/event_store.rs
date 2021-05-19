use std::{cmp::Reverse, convert::TryInto, ops::RangeInclusive};

use actyxos_sdk::{
    Event, EventKey, LamportTimestamp, Metadata, NodeId, Offset, OffsetMap, OffsetOrMin, Payload, StreamId, StreamNr,
    TagSet, Timestamp,
};
use ax_futures_util::{prelude::AxStreamExt, stream::MergeOrdered};
use banyan::FilteredChunk;
use derive_more::{Display, Error};
use futures::{future, stream, Stream, StreamExt, TryStreamExt};
use trees::{
    axtrees::{AxKey, TagsQuery},
    OffsetMapOrMax, TagSubscriptions,
};

use crate::{selection::StreamEventSelection, AxTreeExt, BanyanStore, SwarmOffsets, TT};

#[derive(Clone, Debug, Display, Error)]
pub enum Error {
    #[display(fmt = "Upper bounds must be within the current offsets’ present.")]
    InvalidUpperBounds,
}

pub type PersistenceMeta = (LamportTimestamp, Offset, StreamNr, Timestamp);

#[derive(Clone)]
pub struct EventStore {
    banyan_store: BanyanStore,
}

impl EventStore {
    pub fn new(banyan_store: BanyanStore) -> EventStore {
        EventStore { banyan_store }
    }

    pub fn node_id(&self) -> NodeId {
        self.banyan_store.node_id()
    }

    fn forward_stream(&self, stream_selection: StreamEventSelection) -> impl Stream<Item = Event<Payload>> {
        let stream_id = stream_selection.stream_id;
        debug_assert!(self.banyan_store.has_stream(stream_id));
        debug_assert!(stream_selection.from_exclusive < stream_selection.to_inclusive);
        let (trees, forest) = self.banyan_store.tree_stream(stream_id);
        let range = get_range_inclusive(&stream_selection);
        let query = TagsQuery::new(stream_selection.tag_subscriptions);
        forest
            .stream_trees_chunked(query, trees, range, &|_| ())
            .map_ok(move |chunk| stream::iter(events_from_chunk(stream_id, chunk)))
            .take_while(|x| future::ready(x.is_ok()))
            .filter_map(|x| future::ready(x.ok()))
            .flatten()
    }

    fn backward_stream(&self, stream_selection: StreamEventSelection) -> impl Stream<Item = Reverse<Event<Payload>>> {
        let stream_id = stream_selection.stream_id;
        debug_assert!(stream_selection.from_exclusive < stream_selection.to_inclusive);
        debug_assert!(self.banyan_store.has_stream(stream_id));
        let (trees, forest) = self.banyan_store.tree_stream(stream_id);
        let range = get_range_inclusive(&stream_selection);
        let query = TagsQuery::new(stream_selection.tag_subscriptions);
        forest
            .stream_trees_chunked_reverse(query, trees, range, &|_| ())
            .map_ok(move |chunk| stream::iter(events_from_chunk_rev(stream_id, chunk)))
            .take_while(|x| future::ready(x.is_ok()))
            .filter_map(|x| future::ready(x.ok()))
            .flatten()
    }

    async fn bounded_streams(
        &self,
        tag_subscriptions: TagSubscriptions,
        from_offsets_excluding: Option<OffsetMap>,
        to_offsets_including: OffsetMap,
    ) -> Result<Vec<StreamEventSelection>, Error> {
        let only_local = tag_subscriptions.only_local();
        let this = self.clone();
        let present = self.present().await;
        if present.union(&to_offsets_including) != present {
            return Err(Error::InvalidUpperBounds);
        }
        let from_or_min = from_offsets_excluding.map(OffsetMapOrMax::from).unwrap_or_default();
        let res: Vec<_> = to_offsets_including
            .streams()
            .filter_map(|stream_id| {
                let from = from_or_min.offset(stream_id);
                let to = to_offsets_including.offset(stream_id);
                let local = this.banyan_store.is_local(stream_id);

                if from < to && (!only_local || local) {
                    Some(StreamEventSelection {
                        stream_id,
                        from_exclusive: from,
                        to_inclusive: to,
                        tag_subscriptions: tag_subscriptions.as_tag_sets(local),
                    })
                } else {
                    None
                }
            })
            .collect();
        Ok(res)
    }

    fn unbounded_stream(
        &self,
        stream_id: StreamId,
        tag_subscriptions: &TagSubscriptions,
        from_exclusive: OffsetOrMin,
    ) -> Option<StreamEventSelection> {
        let only_local = tag_subscriptions.only_local();
        let local = self.banyan_store.is_local(stream_id);
        if !only_local || local {
            Some(StreamEventSelection {
                stream_id,
                from_exclusive,
                to_inclusive: OffsetOrMin::MAX,
                tag_subscriptions: tag_subscriptions.as_tag_sets(local),
            })
        } else {
            None
        }
    }

    pub fn offsets(&self) -> impl Stream<Item = SwarmOffsets> {
        self.banyan_store.data.offsets.new_observer()
    }

    pub async fn present(&self) -> OffsetMap {
        self.offsets().next().await.unwrap_or_default().present
    }

    pub async fn persist(&self, events: Vec<(TagSet, Payload)>) -> anyhow::Result<Vec<PersistenceMeta>> {
        let stream_nr = StreamNr::from(0); // TODO
        let timestamp = Timestamp::now();
        let stream = self.banyan_store.get_or_create_own_stream(stream_nr);
        let n = events.len();
        let mut guard = stream.lock().await;
        let mut store = self.banyan_store.lock();
        let mut lamports = store.reserve_lamports(events.len())?.peekable();
        let min_lamport = *lamports.peek().unwrap();
        let kvs = lamports
            .zip(events)
            .map(|(lamport, (tags, payload))| (AxKey::new(tags, lamport, timestamp), payload));
        tracing::debug!("publishing {} events on stream {}", n, stream_nr);
        let min_offset = self.banyan_store.transform_stream(&mut guard, |txn, tree| {
            let result = tree.snapshot().offset();
            txn.extend_unpacked(tree, kvs)?;
            Ok(result)
        })?;

        // We start iteration with 0 below, so this is effectively the offset of the first event.
        let starting_offset = min_offset.succ();
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

    pub async fn bounded_forward(
        &self,
        tag_subscriptions: TagSubscriptions,
        from_offsets_excluding: Option<OffsetMap>,
        to_offsets_including: OffsetMap,
    ) -> Result<impl Stream<Item = Event<Payload>>, Error> {
        let this = self.clone();
        let event_streams = self
            .bounded_streams(tag_subscriptions, from_offsets_excluding, to_offsets_including)
            .await?
            .into_iter()
            .map(|stream_selection| this.forward_stream(stream_selection));
        Ok(MergeOrdered::new_fixed(event_streams))
    }

    pub async fn bounded_forward_per_stream(
        &self,
        tag_subscriptions: TagSubscriptions,
        from_offsets_excluding: Option<OffsetMap>,
        to_offsets_including: OffsetMap,
    ) -> Result<impl Stream<Item = Event<Payload>>, Error> {
        let this = self.clone();
        let event_streams = self
            .bounded_streams(tag_subscriptions, from_offsets_excluding, to_offsets_including)
            .await?
            .into_iter()
            .map(move |stream_selection| this.forward_stream(stream_selection));
        Ok(stream::iter(event_streams).merge_unordered())
    }

    pub async fn bounded_backward(
        &self,
        tag_subscriptions: TagSubscriptions,
        from_offsets_excluding: Option<OffsetMap>,
        to_offsets_including: OffsetMap,
    ) -> Result<impl Stream<Item = Event<Payload>>, Error> {
        let this = self.clone();
        let event_streams = self
            .bounded_streams(tag_subscriptions, from_offsets_excluding, to_offsets_including)
            .await?
            .into_iter()
            .map(|stream_selection| this.backward_stream(stream_selection));
        Ok(MergeOrdered::new_fixed(event_streams).map(|reverse| reverse.0))
    }

    pub fn unbounded_forward_per_stream(
        &self,
        tag_subscriptions: TagSubscriptions,
        from_offsets_excluding: Option<OffsetMap>,
    ) -> impl Stream<Item = Event<Payload>> {
        let this = self.clone();
        let this2 = self.clone();
        let from_or_min = from_offsets_excluding.map(OffsetMapOrMax::from).unwrap_or_default();
        self.banyan_store
            .stream_known_streams()
            .filter_map(move |stream_id| {
                future::ready(this.unbounded_stream(stream_id, &tag_subscriptions, from_or_min.offset(stream_id)))
            })
            .map(move |stream_selection| this2.forward_stream(stream_selection))
            .merge_unordered()
            .boxed()
    }
}

fn get_range_inclusive(selection: &StreamEventSelection) -> RangeInclusive<u64> {
    let min = (selection.from_exclusive - OffsetOrMin::MIN) as u64;
    let max = (selection.to_inclusive - OffsetOrMin::ZERO) as u64;
    min..=max
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

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use actyxos_sdk::{language::TagExpr, service::Order, tags, OffsetOrMin, StreamId};
    use ax_futures_util::stream::Drainer;
    use maplit::btreemap;
    use num_traits::Bounded;
    use quickcheck::Arbitrary;
    use trees::{OffsetMapOrMax, TagSubscription, TagSubscriptions};

    use super::*;
    use crate::{selection::EventSelection, BanyanStore};

    async fn mk_store(name: &'static str) -> EventStore {
        EventStore::new(BanyanStore::test(name).await.unwrap())
    }

    fn offset_map(map: &BTreeMap<StreamId, u32>) -> OffsetMap {
        map.iter()
            .map(|(stream_id, offset)| (*stream_id, Offset::from(*offset)))
            .collect::<BTreeMap<_, _>>()
            .into()
    }

    async fn await_stream_offsets<'a>(
        store: &'a EventStore,
        other_stores: &'a [&EventStore],
        offsets: &BTreeMap<StreamId, u32>,
    ) {
        for other in other_stores {
            store.banyan_store.ipfs().add_address(
                &other.banyan_store.ipfs().local_peer_id(),
                other.banyan_store.ipfs().listeners()[0].clone(),
            );
        }

        let mut waiting_for = offsets.clone();
        let mut present = store.offsets().map(|o| o.present);
        while let Some(incoming) = present.next().await {
            // waiting_for.retain(|stream_id, offset| offset < incoming.offset(stream_id.clone())); // "1.53.0"
            incoming.streams().for_each(|stream_id| {
                if let Some(offset) = waiting_for.get(&stream_id) {
                    if OffsetOrMin::from(*offset) <= incoming.offset(stream_id) {
                        waiting_for.remove(&stream_id);
                    }
                }
            });
            if waiting_for.is_empty() {
                return;
            }
        }
    }

    fn assert_stream(
        stream: impl Stream<Item = Event<Payload>> + 'static,
        expr: &'static str,
        from_offsets_excluding: OffsetMapOrMax,
        to_offsets_including: OffsetMapOrMax,
        len: usize,
        order: Order,
        completed: bool,
    ) {
        let tag_expr = &expr.parse::<TagExpr>().unwrap();
        let tag_subscriptions = tag_expr.into();
        let selection = EventSelection {
            tag_subscriptions,
            from_offsets_excluding,
            to_offsets_including,
        };

        let mut stream = Drainer::new(stream);
        let res: Vec<_> = if completed {
            stream.flatten().collect::<Vec<_>>()
        } else {
            let x = stream.next().unwrap();
            assert_eq!(stream.next(), Some(vec![]));
            x
        };

        assert_eq!(res.len(), len, "actual: {:#?}", res);
        assert!(res.iter().all(|e| selection.matches(e)));

        fn is_sorted(elements: &[impl Ord]) -> bool {
            elements.windows(2).all(|pair| pair[0] <= pair[1])
        }
        match order {
            Order::Asc => assert!(is_sorted(&res)),
            Order::Desc => assert!(is_sorted(&res.iter().map(Reverse).collect::<Vec<_>>())),
            Order::StreamAsc => {} // FIXME
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_forward_stream() {
        let store = mk_store("swarm_test").await;
        let stream_id = store.node_id().stream(0.into());

        store.persist(vec![(tags!(), Payload::empty())]).await.unwrap();

        let mut stream = Drainer::new(store.forward_stream(StreamEventSelection {
            stream_id,
            from_exclusive: OffsetOrMin::MIN,
            to_inclusive: OffsetOrMin::ZERO,
            tag_subscriptions: vec![tags!()],
        }));
        let res = stream.next().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(stream.next(), None);

        let mut stream = Drainer::new(store.forward_stream(StreamEventSelection {
            stream_id,
            from_exclusive: OffsetOrMin::MIN,
            to_inclusive: OffsetOrMin::ZERO,
            tag_subscriptions: vec![tags!("nothing")],
        }));
        assert_eq!(stream.next(), None);

        let mut stream = Drainer::new(store.forward_stream(StreamEventSelection {
            stream_id,
            from_exclusive: OffsetOrMin::MIN,
            to_inclusive: OffsetOrMin::ZERO,
            tag_subscriptions: vec![tags!()],
        }));
        let res = stream.next().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(stream.next(), None); // bounded -> complete

        let mut stream = Drainer::new(store.forward_stream(StreamEventSelection {
            stream_id,
            from_exclusive: OffsetOrMin::MIN,
            to_inclusive: OffsetOrMin::MAX,
            tag_subscriptions: vec![tags!()],
        }));
        let res = stream.next().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(stream.next(), Some(vec![])); // unbounded -> keep running
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_backward_stream() {
        let store = mk_store("swarm_test").await;
        let stream_id = store.node_id().stream(0.into());

        store.persist(vec![(tags!(), Payload::empty())]).await.unwrap();

        let mut stream = Drainer::new(store.backward_stream(StreamEventSelection {
            stream_id,
            from_exclusive: OffsetOrMin::MIN,
            to_inclusive: OffsetOrMin::ZERO,
            tag_subscriptions: vec![tags!()],
        }));
        let res = stream.next().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(stream.next(), None);

        let mut stream = Drainer::new(store.backward_stream(StreamEventSelection {
            stream_id,
            from_exclusive: OffsetOrMin::MIN,
            to_inclusive: OffsetOrMin::ZERO,
            tag_subscriptions: vec![tags!("nothing")],
        }));
        assert_eq!(stream.next(), None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_bounded() {
        let store1 = mk_store("swarm_test1").await;
        let store2 = mk_store("swarm_test2").await;

        let stream_id1 = store1.node_id().stream(0.into());
        let stream_id2 = store2.node_id().stream(0.into());

        store1
            .persist(vec![
                (tags!("test", "test:stream1"), Payload::empty()),
                (tags!("test", "test:stream1"), Payload::empty()),
                (tags!("test", "test:stream1"), Payload::empty()),
            ])
            .await
            .unwrap();
        store2
            .persist(vec![
                (tags!("test", "test:stream2"), Payload::empty()),
                (tags!("test", "test:stream2"), Payload::empty()),
                (tags!("test", "test:stream2"), Payload::empty()),
            ])
            .await
            .unwrap();

        async fn assert_bounded<'a>(
            store: &'a EventStore,
            expr: &'static str,
            from: Option<&'a BTreeMap<StreamId, u32>>,
            to: &'a BTreeMap<StreamId, u32>,
            len: usize,
        ) {
            let from: Option<OffsetMap> = from.map(offset_map).map(Into::into);
            let to: OffsetMap = offset_map(to);
            let tag_expr = &expr.parse::<TagExpr>().unwrap();
            let tag_subscriptions: TagSubscriptions = tag_expr.into();

            let forward = store
                .bounded_forward(tag_subscriptions.clone(), from.clone(), to.clone())
                .await
                .unwrap();
            assert_stream(
                forward,
                expr,
                from.clone().unwrap_or_default().into(),
                to.clone().into(),
                len,
                Order::Asc,
                true,
            );

            let backward = store
                .bounded_backward(tag_subscriptions, from.clone(), to.clone())
                .await
                .unwrap();
            assert_stream(
                backward,
                expr,
                from.unwrap_or_default().into(),
                to.into(),
                len,
                Order::Desc,
                true,
            );
        }

        let max = btreemap! {
          stream_id1 => 2,
          stream_id2 => 2,
        };
        let _ = await_stream_offsets(&store1, &[&store2], &max).await;

        // all
        assert_bounded(&store1, "'test'", None, &max, 6).await;

        // stream1
        assert_bounded(&store1, "isLocal", None, &max, 3).await;
        assert_bounded(&store1, "'test'", None, &btreemap! { stream_id1 => 2 }, 3).await;
        assert_bounded(
            &store1,
            "'test'",
            Some(&btreemap! { stream_id1 => 0u32 }),
            &btreemap! { stream_id1 => 1u32 },
            1,
        )
        .await;

        // stream2
        assert_bounded(&store1, "'test:stream2'", None, &max, 3).await;
        assert_bounded(&store1, "'test'", None, &btreemap! { stream_id2 => 2 }, 3).await;
        assert_bounded(
            &store1,
            "'test'",
            Some(&btreemap! { stream_id2 => 0u32 }),
            &btreemap! { stream_id2 => 1u32 },
            1,
        )
        .await;

        let unknown = store1
            .bounded_forward(
                (&"'test'".parse::<TagExpr>().unwrap()).into(),
                None,
                offset_map(&btreemap! {
                    "Kh8od22U1f.2S7wHoVCnmJaKWX/6.e2dSlEk2K3Jia6-0".parse::<StreamId>().unwrap() => 0
                }),
            )
            .await;
        assert!(matches!(unknown, Err(Error::InvalidUpperBounds)));

        let exceeding_present = store1
            .bounded_forward(
                (&"'test'".parse::<TagExpr>().unwrap()).into(),
                None,
                offset_map(&btreemap! { stream_id1 => 42 }),
            )
            .await;
        assert!(matches!(exceeding_present, Err(Error::InvalidUpperBounds)));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_unbounded_forward() {
        let store1 = mk_store("swarm_test1").await;
        let store2 = mk_store("swarm_test2").await;

        let stream_id1 = store1.node_id().stream(0.into());
        let stream_id2 = store2.node_id().stream(0.into());

        store1
            .persist(vec![(tags!("test:unbounded:forward"), Payload::empty())])
            .await
            .unwrap();

        async fn assert_unbounded<'a>(
            stream: impl Stream<Item = Event<Payload>> + 'static,
            expr: &'static str,
            from: Option<&'a BTreeMap<StreamId, u32>>,
            len: usize,
        ) {
            assert_stream(
                stream,
                expr,
                from.map(|entries| offset_map(entries)).unwrap_or_default().into(),
                OffsetMapOrMax::max_value(),
                len,
                Order::StreamAsc,
                false,
            );
        }

        let store1_clone = store1.clone();
        let store2_clone = store2.clone();
        let handle = tokio::spawn(async move {
            let store_rx = mk_store("swarm_test_rx").await;
            let tag_subscriptions = TagSubscriptions::new(vec![TagSubscription::new(tags!("test:unbounded:forward"))]);
            let from = btreemap! { stream_id1 => 0 };
            // stream1 is below range and stream2 non-existant at this point
            let stream = store_rx.unbounded_forward_per_stream(tag_subscriptions, Some(offset_map(&from)));
            let _ = await_stream_offsets(
                &store_rx,
                &[&store1_clone, &store2_clone],
                &btreemap! {stream_id1 => 1, stream_id2 => 0 },
            )
            .await;
            assert_unbounded(stream, "'test:unbounded:forward'", Some(&from), 2).await;
        });

        store1
            .persist(vec![(tags!("test:unbounded:forward"), Payload::empty())])
            .await
            .unwrap();
        store2
            .persist(vec![(tags!("test:unbounded:forward"), Payload::empty())])
            .await
            .unwrap();

        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_unbounded_streams() {
        let local = mk_store("unbounded_streams_local").await;
        let remote = mk_store("unbounded_streams_remote").await;

        // create dummy streams
        local.persist(vec![(tags!(), Payload::empty())]).await.unwrap();
        remote.persist(vec![(tags!(), Payload::empty())]).await.unwrap();

        let local_stream = local.node_id().stream(0.into());
        let remote_stream = remote.node_id().stream(0.into());

        await_stream_offsets(
            &local,
            &[&remote],
            &btreemap! {
              local_stream => 0,
              remote_stream => 0,
            },
        )
        .await;

        let test_selection = |stream_id: StreamId, tag_expr: &'static str, expected: Option<Vec<TagSet>>| {
            let tag_subscriptions = TagSubscriptions::from(&tag_expr.parse::<TagExpr>().unwrap());
            let actual = local
                .unbounded_stream(stream_id, &tag_subscriptions, OffsetOrMin::MIN)
                .map(|selection| selection.tag_subscriptions);
            assert_eq!(actual, expected, "tag_expr: {:?}", tag_expr);
        };

        test_selection(local_stream, "isLocal", Some(vec![]));
        test_selection(local_stream, "isLocal & 'a'", Some(vec![tags!("a")]));
        test_selection(local_stream, "isLocal | 'a'", Some(vec![]));
        test_selection(local_stream, "isLocal & 'b' | 'a'", Some(vec![tags!("a"), tags!("b")]));
        test_selection(local_stream, "'a'", Some(vec![tags!("a")]));

        test_selection(remote_stream, "isLocal", None);
        test_selection(remote_stream, "isLocal & 'a'", None);
        test_selection(remote_stream, "isLocal | 'a'", Some(vec![tags!("a")]));
        test_selection(remote_stream, "isLocal & 'b' | 'a'", Some(vec![tags!("a")]));
        test_selection(remote_stream, "'a'", Some(vec![tags!("a")]));
    }

    #[tokio::test]
    async fn should_stream_offsets() -> anyhow::Result<()> {
        fn test_offsets(store: &EventStore, stream: StreamId, offset: Offset) {
            let mut offsets = Drainer::new(store.offsets());

            // Inject root update from `stream`
            store.banyan_store.update_highest_seen(stream, offset.into());

            let nxt = offsets.next().unwrap().last().cloned().unwrap();
            assert!(nxt.present.streams().all(|x| x != stream));
            assert_eq!(nxt.replication_target.offset(stream), OffsetOrMin::from(offset));

            // Inject validation of `stream` with `offset - 1`
            if let Some(pred) = offset.pred() {
                store.banyan_store.update_present(stream, pred.into());
                let nxt = offsets.next().unwrap().last().cloned().unwrap();
                assert_eq!(nxt.present.offset(stream), OffsetOrMin::from(pred));
                assert_eq!(nxt.replication_target.offset(stream), OffsetOrMin::from(offset));
            }

            // Inject validation of `stream` with `offset`
            store.banyan_store.update_present(stream, offset.into());
            let nxt = offsets.next().unwrap().last().cloned().unwrap();
            assert_eq!(nxt.present.offset(stream), OffsetOrMin::from(offset));
            assert_eq!(nxt.replication_target.offset(stream), OffsetOrMin::from(offset));
        }

        let store = mk_store("offset_stream").await;
        let store_node_id = store.node_id();
        let mut offsets = Drainer::new(store.offsets());

        // Initially only own streams
        let nxt = offsets.next().unwrap().last().cloned().unwrap();
        assert!(nxt.present.streams().all(|x| x.node_id() == store_node_id));
        assert_eq!(nxt.replication_target, Default::default());

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
}
