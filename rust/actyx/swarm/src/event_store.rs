use std::{cmp::Reverse, convert::TryInto, ops::RangeInclusive};

use actyx_sdk::{
    language::TagExpr, AppId, Event, EventKey, LamportTimestamp, Metadata, NodeId, Offset, OffsetMap, OffsetOrMin,
    Payload, StreamId, StreamNr, TagSet, Timestamp,
};
use ax_futures_util::{prelude::AxStreamExt, stream::MergeOrdered};
use banyan::FilteredChunk;
use derive_more::{Display, Error};
use futures::{future, stream, Stream, StreamExt, TryStreamExt};
use trees::{axtrees::AxKey, query::TagsQuery};

use crate::{selection::StreamEventSelection, AppendMeta, BanyanStore, SwarmOffsets};

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
        from_offsets_excluding: OffsetMap,
        to_offsets_including: OffsetMap,
    ) -> Result<Vec<StreamEventSelection>, Error> {
        let this = self.clone();
        let present = self.current_offsets().present;
        if present.union(&to_offsets_including) != present {
            return Err(Error::InvalidUpperBounds);
        }
        let mk_tags_query = TagsQuery::from_expr(tag_expr);
        let res: Vec<_> = to_offsets_including
            .streams()
            .filter_map(|stream_id| {
                let local = this.banyan_store.is_local(stream_id);
                let from_exclusive = from_offsets_excluding.offset(stream_id);
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

    #[cfg(test)]
    fn offsets(&self) -> impl Stream<Item = SwarmOffsets> {
        self.banyan_store.data.offsets.new_observer()
    }

    pub fn current_offsets(&self) -> SwarmOffsets {
        self.banyan_store.data.offsets.get_cloned()
    }

    pub async fn persist(&self, app_id: AppId, events: Vec<(TagSet, Payload)>) -> anyhow::Result<Vec<PersistenceMeta>> {
        if events.is_empty() {
            return Ok(vec![]);
        }
        let stream_nr = StreamNr::from(0); // TODO
        let n = events.len();
        if n == 0 {
            return Ok(vec![]);
        }
        let AppendMeta {
            min_lamport,
            min_offset,
            timestamp,
            ..
        } = self.banyan_store.append(stream_nr, app_id, events).await?;
        let keys = (0..n)
            .map(|i| {
                let i = i as u64;
                let lamport = min_lamport + i;
                let offset = min_offset.increase(i).unwrap();
                (lamport, offset, stream_nr, timestamp)
            })
            .collect();
        Ok(keys)
    }

    pub async fn bounded_forward(
        &self,
        tag_expr: &TagExpr,
        from_offsets_excluding: OffsetMap,
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
        from_offsets_excluding: OffsetMap,
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
        from_offsets_excluding: OffsetMap,
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
        from_offsets_excluding: OffsetMap,
    ) -> impl Stream<Item = Event<Payload>> {
        let this = self.clone();
        let mk_tags_query = TagsQuery::from_expr(&tag_expr);
        let banyan_store = self.banyan_store.clone();
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
                        from_exclusive: from_offsets_excluding.offset(stream_id),
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

fn to_ev(offset: u64, key: AxKey, stream: StreamId, payload: Payload) -> Option<Event<Payload>> {
    Some(Event {
        payload,
        key: EventKey {
            lamport: key.lamport(),
            offset: offset.try_into().expect("invalid offset value"),
            stream,
        },
        meta: Metadata {
            timestamp: key.time(),
            app_id: key.app_id()?,
            tags: key.into_app_tags(),
        },
    })
}

/// Take a block of banyan events and convert them into events.
fn events_from_chunk(stream_id: StreamId, chunk: FilteredChunk<(u64, AxKey, Payload), ()>) -> Vec<Event<Payload>> {
    chunk
        .data
        .into_iter()
        .filter_map(move |(offset, key, payload)| to_ev(offset, key, stream_id, payload))
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
        .filter_map(move |(offset, key, payload)| to_ev(offset, key, stream_id, payload).map(Reverse))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use actyx_sdk::{
        app_id,
        language::{TagAtom, TagExpr},
        service::Order,
        tag, tags, OffsetOrMin, StreamId, Tag,
    };
    use ax_futures_util::stream::Drainer;
    use futures::future::try_join_all;
    use maplit::btreemap;
    use quickcheck::Arbitrary;
    use rand::{thread_rng, Rng};

    use super::*;
    use crate::{selection::EventSelection, BanyanStore};

    async fn mk_store(name: &'static str) -> EventStore {
        EventStore::new(BanyanStore::test(name).await.unwrap())
    }

    fn app_id() -> AppId {
        app_id!("test")
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
        let app_id = app_id!("test_forward_stream");

        store
            .persist(app_id.clone(), vec![(tags!(), Payload::empty())])
            .await
            .unwrap();

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
        assert_eq!(res[0].meta.app_id, app_id);
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
        let app_id = app_id!("test_backward_stream");

        store
            .persist(app_id.clone(), vec![(tags!(), Payload::empty())])
            .await
            .unwrap();

        let mut stream = Drainer::new(store.backward_stream(StreamEventSelection {
            stream_id,
            from_exclusive: OffsetOrMin::MIN,
            to_inclusive: OffsetOrMin::ZERO,
            tags_query: TagsQuery::all(),
        }));
        let res = stream.next().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].0.meta.app_id, app_id);
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
            .persist(
                app_id(),
                vec![
                    (tags!("test", "test:stream1"), Payload::empty()),
                    (tags!("test", "test:stream1"), Payload::empty()),
                    (tags!("test", "test:stream1"), Payload::empty()),
                ],
            )
            .await
            .unwrap();
        store2
            .persist(
                app_id(),
                vec![
                    (tags!("test", "test:stream2"), Payload::empty()),
                    (tags!("test", "test:stream2"), Payload::empty()),
                    (tags!("test", "test:stream2"), Payload::empty()),
                ],
            )
            .await
            .unwrap();

        async fn assert_bounded<'a>(
            store: &'a EventStore,
            expr: &'static str,
            from: Option<&'a BTreeMap<StreamId, u32>>,
            to: &'a BTreeMap<StreamId, u32>,
            len: usize,
        ) {
            let from: OffsetMap = from.map(offset_map).unwrap_or_default();
            let to: OffsetMap = offset_map(to);
            let expr = &expr.parse::<TagExpr>().unwrap();
            let selection = EventSelection {
                from_offsets_excluding: from.clone(),
                to_offsets_including: to.clone(),
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
                OffsetMap::default(),
                offset_map(&btreemap! {
                  "Kh8od22U1f.2S7wHoVCnmJaKWX/6.e2dSlEk2K3Jia6-0".parse::<StreamId>().unwrap() => 0
                }),
            )
            .await;
        assert!(matches!(unknown, Err(Error::InvalidUpperBounds)));

        let exceeding_present = store1
            .bounded_forward(
                &TagExpr::Atom(TagAtom::AllEvents),
                OffsetMap::default(),
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
            .persist(app_id(), vec![(tags!("test:unbounded:forward"), Payload::empty())])
            .await
            .unwrap();

        async fn assert_unbounded<'a>(
            node_id: NodeId,
            stream: impl Stream<Item = Event<Payload>> + 'static,
            expr: &'static str,
            from: OffsetMap,
            to: OffsetMap,
            len: usize,
        ) {
            let expr = expr.parse::<TagExpr>().unwrap();
            assert_stream(
                node_id,
                stream,
                EventSelection {
                    tag_expr: expr,
                    from_offsets_excluding: from,
                    to_offsets_including: to,
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
            let from = offset_map(&btreemap! { stream_id1 => 0 });
            let to = offset_map(&btreemap! { stream_id1 => u32::MAX, stream_id2 => u32::MAX });
            // stream1 is below range and stream2 non-existant at this point
            let stream = store_rx.unbounded_forward_per_stream(tag_expr, from.clone());
            let _ = await_stream_offsets(
                &store_rx,
                &[&store1_clone, &store2_clone],
                &btreemap! {stream_id1 => 1, stream_id2 => 0 },
            )
            .await;
            assert_unbounded(store_rx.node_id(), stream, "'test:unbounded:forward'", from, to, 2).await;
        });

        store1
            .persist(app_id(), vec![(tags!("test:unbounded:forward"), Payload::empty())])
            .await
            .unwrap();
        store2
            .persist(app_id(), vec![(tags!("test:unbounded:forward"), Payload::empty())])
            .await
            .unwrap();

        handle.await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_pubsub() {
        let n = 100;

        let random_range = move || {
            let i1 = thread_rng().gen_range(0..n) as usize;
            let i2 = thread_rng().gen_range(0..n) as usize;
            let from = i1.min(i2);
            let to = i1.max(i2);
            from..=to
        };

        fn mk_tag(i: u32) -> TagSet {
            std::iter::once(
                (if i % 2 == 0 { "evn" } else { "odd" })
                    .to_owned()
                    .parse::<Tag>()
                    .unwrap(),
            )
            .collect()
        }

        let offsets: Vec<(Offset, TagSet)> = (0..n).into_iter().map(|i| (i.into(), mk_tag(i))).collect();
        let store = mk_store("pubsub").await;
        let stream_id = store.node_id().stream(0.into());

        let mut handles = Vec::new();
        for i in 0..n {
            let (_, offset, _, _) = store
                .persist(app_id(), vec![(mk_tag(i), Payload::empty())])
                .await
                .unwrap()[0];
            assert_eq!(offset, Offset::from(i as u32));

            let store = store.clone();
            let offsets = offsets.clone();
            let handle_sub = tokio::spawn(async move {
                // app tags, for comparison with the result
                let tags = mk_tag(i);
                // tags with prefix, as they actually appear on the tree
                let scoped_tags = tags.clone().into();
                let tags_query = TagsQuery::new(vec![scoped_tags]);
                let range = random_range();
                let actual = store
                    .forward_stream(StreamEventSelection {
                        stream_id,
                        from_exclusive: OffsetOrMin::from(*range.start() as i64 - 1),
                        to_inclusive: OffsetOrMin::from(*range.end() as i64),
                        tags_query,
                    })
                    .map(|e| e.key.offset)
                    .collect::<Vec<_>>()
                    .await;

                let expected: Vec<_> = offsets[range]
                    .iter()
                    .filter_map(|(o, t)| if t == &tags { Some(*o) } else { None })
                    .collect();
                assert_eq!(actual, expected);
            });
            handles.push(handle_sub);
        }
        try_join_all(handles).await.unwrap();
    }

    #[tokio::test]
    async fn should_stream_offsets() -> anyhow::Result<()> {
        fn test_offsets(store: &EventStore, stream: StreamId, offset: Offset) {
            let mut offsets = Drainer::new(store.offsets());

            // Inject root update from `stream`
            store.banyan_store.update_highest_seen(stream, offset);

            let nxt = offsets.next().unwrap().last().cloned().unwrap();
            assert!(nxt.present.streams().all(|x| x != stream));
            assert_eq!(nxt.replication_target.offset(stream), OffsetOrMin::from(offset));

            // Inject validation of `stream` with `offset - 1`
            if let Some(pred) = offset.pred() {
                store.banyan_store.update_present(stream, pred);
                let nxt = offsets.next().unwrap().last().cloned().unwrap();
                assert_eq!(nxt.present.offset(stream), OffsetOrMin::from(pred));
                assert_eq!(nxt.replication_target.offset(stream), OffsetOrMin::from(offset));
            }

            // Inject validation of `stream` with `offset`
            store.banyan_store.update_present(stream, offset);
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
