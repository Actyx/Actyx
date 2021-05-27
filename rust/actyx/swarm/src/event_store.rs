use std::{cmp::Reverse, convert::TryInto, ops::RangeInclusive};

use actyxos_sdk::{
    language::TagExpr, Event, EventKey, LamportTimestamp, Metadata, NodeId, Offset, OffsetMap, OffsetOrMin, Payload,
    StreamId, StreamNr, TagSet, Timestamp,
};
use ax_futures_util::{prelude::AxStreamExt, stream::MergeOrdered};
use banyan::FilteredChunk;
use derive_more::{Display, Error};
use futures::{future, stream, Stream, StreamExt, TryStreamExt};
use trees::{axtrees::AxKey, query::TagsQuery, OffsetMapOrMax};

use crate::{selection::StreamEventSelection, AxTreeExt, BanyanStore, SwarmOffsets, MAX_TREE_LEVEL};

#[derive(Clone, Debug, Display, Error)]
pub enum Error {
    #[display(fmt = "Upper bounds must be within the current offsets’ present.")]
    InvalidUpperBounds,
}

pub type PersistenceMeta = (LamportTimestamp, Offset, StreamNr, Timestamp);

/// Wraps a [BanyanStore] and provides functionality for persisting events as well as receiving bounded and
/// unbounded sets of events for queries across multiple streams with varying order guarantees.
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

    fn forward_stream(&self, selection: StreamEventSelection) -> impl Stream<Item = Event<Payload>> {
        let stream_id = selection.stream_id;
        debug_assert!(self.banyan_store.has_stream(stream_id));
        debug_assert!(selection.from_exclusive < selection.to_inclusive);
        let trees = self.banyan_store.tree_stream(stream_id);
        let range = get_range_inclusive(&selection);
        self.banyan_store
            .data
            .forest
            .stream_trees_chunked(selection.tags_query, trees, range, &|_| ())
            .map_ok(move |chunk| stream::iter(events_from_chunk(stream_id, chunk)))
            .take_while(|x| future::ready(x.is_ok()))
            .filter_map(|x| future::ready(x.ok()))
            .flatten()
    }

    fn backward_stream(&self, selection: StreamEventSelection) -> impl Stream<Item = Reverse<Event<Payload>>> {
        let stream_id = selection.stream_id;
        debug_assert!(selection.from_exclusive < selection.to_inclusive);
        debug_assert!(self.banyan_store.has_stream(stream_id));
        let trees = self.banyan_store.tree_stream(stream_id);
        let range = get_range_inclusive(&selection);
        self.banyan_store
            .data
            .forest
            .stream_trees_chunked_reverse(selection.tags_query, trees, range, &|_| ())
            .map_ok(move |chunk| stream::iter(events_from_chunk_rev(stream_id, chunk)))
            .take_while(|x| future::ready(x.is_ok()))
            .filter_map(|x| future::ready(x.ok()))
            .flatten()
    }

    async fn bounded_streams(
        &self,
        tag_expr: &TagExpr,
        from_offsets_excluding: Option<OffsetMap>,
        to_offsets_including: OffsetMap,
    ) -> Result<Vec<StreamEventSelection>, Error> {
        let this = self.clone();
        let present = self.present().await;
        if present.union(&to_offsets_including) != present {
            return Err(Error::InvalidUpperBounds);
        }
        let from_or_min = from_offsets_excluding.map(OffsetMapOrMax::from).unwrap_or_default();
        let mk_tags_query = TagsQuery::from_expr(tag_expr);
        let res: Vec<_> = to_offsets_including
            .streams()
            .filter_map(|stream_id| {
                let local = this.banyan_store.is_local(stream_id);
                let from_exclusive = from_or_min.offset(stream_id);
                let to_inclusive = to_offsets_including.offset(stream_id);
                if from_exclusive >= to_inclusive {
                    return None;
                }
                let tags_query = mk_tags_query(local);
                if tags_query.is_empty() {
                    return None;
                }
                Some(StreamEventSelection {
                    stream_id,
                    from_exclusive,
                    to_inclusive,
                    tags_query,
                })
            })
            .collect();
        Ok(res)
    }

    pub fn offsets(&self) -> impl Stream<Item = SwarmOffsets> {
        self.banyan_store.data.offsets.new_observer()
    }

    pub async fn present(&self) -> OffsetMap {
        self.offsets().next().await.expect("offset stream stopped").present
    }

    pub async fn persist(&self, events: Vec<(TagSet, Payload)>) -> anyhow::Result<Vec<PersistenceMeta>> {
        let stream_nr = StreamNr::from(0); // TODO
        let timestamp = Timestamp::now();
        let stream = self.banyan_store.get_or_create_own_stream(stream_nr)?;
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
            let snapshot = tree.snapshot();
            if snapshot.level() > MAX_TREE_LEVEL {
                txn.extend(tree, kvs)?;
            } else {
                txn.extend_unpacked(tree, kvs)?;
            }
            Ok(snapshot.offset())
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
        tag_expr: &TagExpr,
        from_offsets_excluding: Option<OffsetMap>,
        to_offsets_including: OffsetMap,
    ) -> Result<impl Stream<Item = Event<Payload>>, Error> {
        let this = self.clone();
        let event_streams = self
            .bounded_streams(tag_expr, from_offsets_excluding, to_offsets_including)
            .await?
            .into_iter()
            .map(|selection| this.forward_stream(selection));
        Ok(MergeOrdered::new_fixed(event_streams))
    }

    pub async fn bounded_forward_per_stream(
        &self,
        tag_expr: &TagExpr,
        from_offsets_excluding: Option<OffsetMap>,
        to_offsets_including: OffsetMap,
    ) -> Result<impl Stream<Item = Event<Payload>>, Error> {
        let this = self.clone();
        let event_streams = self
            .bounded_streams(tag_expr, from_offsets_excluding, to_offsets_including)
            .await?
            .into_iter()
            .map(move |selection| this.forward_stream(selection));
        Ok(stream::iter(event_streams).merge_unordered())
    }

    pub async fn bounded_backward(
        &self,
        tag_expr: &TagExpr,
        from_offsets_excluding: Option<OffsetMap>,
        to_offsets_including: OffsetMap,
    ) -> Result<impl Stream<Item = Event<Payload>>, Error> {
        let this = self.clone();
        let event_streams = self
            .bounded_streams(tag_expr, from_offsets_excluding, to_offsets_including)
            .await?
            .into_iter()
            .map(|selection| this.backward_stream(selection));
        Ok(MergeOrdered::new_fixed(event_streams).map(|reverse| reverse.0))
    }

    pub fn unbounded_forward_per_stream(
        &self,
        tag_expr: &TagExpr,
        from_offsets_excluding: Option<OffsetMap>,
    ) -> impl Stream<Item = Event<Payload>> {
        let this = self.clone();
        let mk_tags_query = TagsQuery::from_expr(&tag_expr);
        let banyan_store = self.banyan_store.clone();
        let from_or_min = from_offsets_excluding.map(OffsetMapOrMax::from).unwrap_or_default();
        self.banyan_store
            .stream_known_streams()
            .filter_map(move |stream_id| {
                let local = banyan_store.is_local(stream_id);
                let tags_query = mk_tags_query(local);
                future::ready(if tags_query.is_empty() {
                    None
                } else {
                    Some(StreamEventSelection {
                        stream_id,
                        from_exclusive: from_or_min.offset(stream_id),
                        to_inclusive: OffsetOrMin::MAX,
                        tags_query,
                    })
                })
            })
            .map(move |selection| this.forward_stream(selection))
            .merge_unordered()
    }
}

fn get_range_inclusive(selection: &StreamEventSelection) -> RangeInclusive<u64> {
    use std::convert::TryFrom;
    let min = u64::try_from(selection.from_exclusive - OffsetOrMin::MIN).expect("negative value");
    let max = u64::try_from(selection.to_inclusive - OffsetOrMin::ZERO).expect("negative value");
    min..=max
}

fn to_ev(offset: u64, key: AxKey, stream: StreamId, payload: Payload) -> Event<Payload> {
    Event {
        payload,
        key: EventKey {
            lamport: key.lamport(),
            offset: offset.try_into().expect("invalid offset value"),
            stream,
        },
        meta: Metadata {
            timestamp: key.time(),
            tags: key.into_tags(),
        },
    }
}

/// Take a block of banyan events and convert them into events.
fn events_from_chunk(stream_id: StreamId, chunk: FilteredChunk<(u64, AxKey, Payload), ()>) -> Vec<Event<Payload>> {
    chunk
        .data
        .into_iter()
        .map(move |(offset, key, payload)| to_ev(offset, key, stream_id, payload))
        .collect()
}

/// Take a block of banyan events and convert them into events, reversing them.
fn events_from_chunk_rev(
    stream_id: StreamId,
    chunk: FilteredChunk<(u64, AxKey, Payload), ()>,
) -> Vec<Reverse<Event<Payload>>> {
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

    use actyxos_sdk::{
        language::{TagAtom, TagExpr},
        service::Order,
        tag, tags, OffsetOrMin, StreamId,
    };
    use ax_futures_util::stream::Drainer;
    use futures::future::try_join_all;
    use maplit::btreemap;
    use num_traits::Bounded;
    use quickcheck::Arbitrary;
    use rand::{thread_rng, Rng};
    use trees::OffsetMapOrMax;

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
        node_id: NodeId,
        stream: impl Stream<Item = Event<Payload>> + 'static,
        selection: EventSelection,
        len: usize,
        order: Order,
        completed: bool,
    ) {
        let mut stream = Drainer::new(stream);
        let res: Vec<_> = if completed {
            stream.flatten().collect::<Vec<_>>()
        } else {
            let x = stream.next().unwrap();
            assert_eq!(stream.next(), Some(vec![]));
            x
        };

        assert_eq!(res.len(), len, "actual: {:#?}", res);

        fn matches(selection: EventSelection, node_id: NodeId) -> impl FnMut(&Event<Payload>) -> bool {
            move |e| selection.matches(e.key.stream.node_id() == node_id, &e)
        }
        assert!(res.iter().all(matches(selection, node_id)));

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
            tags_query: TagsQuery::all(),
        }));
        let res = stream.next().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(stream.next(), None);

        let mut stream = Drainer::new(store.forward_stream(StreamEventSelection {
            stream_id,
            from_exclusive: OffsetOrMin::MIN,
            to_inclusive: OffsetOrMin::ZERO,
            tags_query: TagsQuery::empty(),
        }));
        assert_eq!(stream.next(), None);

        let mut stream = Drainer::new(store.forward_stream(StreamEventSelection {
            stream_id,
            from_exclusive: OffsetOrMin::MIN,
            to_inclusive: OffsetOrMin::ZERO,
            tags_query: TagsQuery::all(),
        }));
        let res = stream.next().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(stream.next(), None); // bounded -> complete

        let mut stream = Drainer::new(store.forward_stream(StreamEventSelection {
            stream_id,
            from_exclusive: OffsetOrMin::MIN,
            to_inclusive: OffsetOrMin::MAX,
            tags_query: TagsQuery::all(),
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
            tags_query: TagsQuery::all(),
        }));
        let res = stream.next().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(stream.next(), None);

        let mut stream = Drainer::new(store.backward_stream(StreamEventSelection {
            stream_id,
            from_exclusive: OffsetOrMin::MIN,
            to_inclusive: OffsetOrMin::ZERO,
            tags_query: TagsQuery::empty(),
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
            let expr = &expr.parse::<TagExpr>().unwrap();
            let selection = EventSelection {
                from_offsets_excluding: from.clone().unwrap_or_default().into(),
                to_offsets_including: to.clone().into(),
                tag_expr: expr.clone(),
            };

            let forward = store.bounded_forward(expr, from.clone(), to.clone()).await.unwrap();
            assert_stream(store.node_id(), forward, selection.clone(), len, Order::Asc, true);

            let backward = store.bounded_backward(expr, from.clone(), to.clone()).await.unwrap();
            assert_stream(store.node_id(), backward, selection, len, Order::Desc, true);
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
                &TagExpr::Atom(TagAtom::AllEvents),
                None,
                offset_map(&btreemap! {
                  "Kh8od22U1f.2S7wHoVCnmJaKWX/6.e2dSlEk2K3Jia6-0".parse::<StreamId>().unwrap() => 0
                }),
            )
            .await;
        assert!(matches!(unknown, Err(Error::InvalidUpperBounds)));

        let exceeding_present = store1
            .bounded_forward(
                &TagExpr::Atom(TagAtom::AllEvents),
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
            node_id: NodeId,
            stream: impl Stream<Item = Event<Payload>> + 'static,
            expr: &'static str,
            from: Option<&'a BTreeMap<StreamId, u32>>,
            len: usize,
        ) {
            let expr = expr.parse::<TagExpr>().unwrap();
            assert_stream(
                node_id,
                stream,
                EventSelection {
                    tag_expr: expr,
                    from_offsets_excluding: from.map(|entries| offset_map(entries)).unwrap_or_default().into(),
                    to_offsets_including: OffsetMapOrMax::max_value(),
                },
                len,
                Order::StreamAsc,
                false,
            );
        }

        let store1_clone = store1.clone();
        let store2_clone = store2.clone();
        let handle = tokio::spawn(async move {
            let store_rx = mk_store("swarm_test_rx").await;
            let tag_expr = &TagExpr::Atom(TagAtom::Tag(tag!("test:unbounded:forward")));
            let from = btreemap! { stream_id1 => 0 };
            // stream1 is below range and stream2 non-existant at this point
            let stream = store_rx.unbounded_forward_per_stream(tag_expr, Some(offset_map(&from)));
            let _ = await_stream_offsets(
                &store_rx,
                &[&store1_clone, &store2_clone],
                &btreemap! {stream_id1 => 1, stream_id2 => 0 },
            )
            .await;
            assert_unbounded(store_rx.node_id(), stream, "'test:unbounded:forward'", Some(&from), 2).await;
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_pubsub() {
        fn payload(i: i32) -> Payload {
            Payload::from_json_value(serde_json::json!(i)).unwrap()
        }

        let n = 200;
        let payloads: Vec<_> = (0..n).into_iter().map(payload).collect();
        let store = mk_store("pubsub").await;
        let stream_id = store.node_id().stream(0.into());
        store.persist(vec![(tags!(), payload(0))]).await.unwrap(); // create stream
        let mut handles = Vec::new();

        for i in 1..n {
            let store_sub = store.clone();
            let expected = payloads.clone();
            let handle_sub = tokio::spawn(async move {
                let i1 = thread_rng().gen_range(0..n) as usize;
                let i2 = thread_rng().gen_range(0..n) as usize;

                let from = i1.min(i2);
                let to = i1.max(i2);

                let from_exclusive = OffsetOrMin::from(from as i64 - 1);
                let to_inclusive = OffsetOrMin::from(to as i64);
                let res = store_sub
                    .forward_stream(StreamEventSelection {
                        stream_id,
                        from_exclusive,
                        to_inclusive,
                        tags_query: TagsQuery::all(),
                    })
                    .map(|e| e.payload)
                    .collect::<Vec<_>>()
                    .await;

                assert_eq!(res, expected[from..=to]);
                println!("i{}: {:?} ✔", i, from..=to);
            });
            handles.push(handle_sub);

            let store_pub = store.clone();
            let handle_pub = tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(i as u64 * 100)).await;
                let (_, offset, _, _) = store_pub.persist(vec![(tags!(), payload(i as i32))]).await.unwrap()[0];
                assert_eq!(offset, Offset::from(i as u32))
            });
            handles.push(handle_pub);
        }
        try_join_all(handles).await.unwrap();
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
        // TODO: enforce present vs. replication target invariants https://github.com/Actyx/Cosmos/issues/6720
        // assert_eq!(nxt.replication_target, Default::default());

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
