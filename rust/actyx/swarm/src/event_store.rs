use std::cmp::Reverse;

use actyxos_sdk::{
    service::OffsetsResponse, Event, LamportTimestamp, Offset, OffsetOrMin, Payload, StreamId, StreamNr, TagSet,
    Timestamp,
};
use ax_futures_util::{prelude::AxStreamExt, stream::MergeOrdered};
use derive_more::Display;
use futures::{
    future::{self, BoxFuture},
    stream::{self, BoxStream},
    FutureExt, StreamExt, TryFutureExt,
};
use trees::OffsetMapOrMax;

use crate::selection::{EventSelection, StreamEventSelection};

#[derive(Clone, Debug, Display)]
pub struct EventStoreError;
impl std::error::Error for EventStoreError {}

pub type EventStreamOrError = BoxFuture<'static, Result<BoxStream<'static, Event<Payload>>, EventStoreError>>;
pub type EventStreamReverseOrError =
    BoxFuture<'static, Result<BoxStream<'static, Reverse<Event<Payload>>>, EventStoreError>>;
pub type PersistenceMeta = (LamportTimestamp, Offset, StreamNr, Timestamp);

pub trait EventStore: Clone + Sized + Sync + Send + 'static {
    fn is_local(&self, stream_id: StreamId) -> bool;

    fn has_stream(&self, stream_id: StreamId) -> bool;

    fn known_streams(&self) -> BoxStream<'static, StreamId>;

    fn forward_stream(&self, stream_selection: StreamEventSelection) -> EventStreamOrError;

    fn backward_stream(&self, stream_selection: StreamEventSelection) -> EventStreamReverseOrError;

    fn bounded_streams(&self, selection: EventSelection, present: OffsetMapOrMax) -> Vec<StreamEventSelection> {
        let EventSelection {
            from_offsets_excluding,
            to_offsets_including,
            tag_subscriptions,
        } = selection;
        let only_local = tag_subscriptions.only_local();
        to_offsets_including
            .streams()
            .filter_map(|stream_id| {
                let from = from_offsets_excluding.offset(stream_id);
                let to = to_offsets_including.offset(stream_id).min(present.offset(stream_id));
                let local = self.is_local(stream_id);

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
            .collect()
    }

    fn unbounded_stream(&self, selection: &EventSelection, stream_id: StreamId) -> Option<StreamEventSelection> {
        let only_local = selection.tag_subscriptions.only_local();
        let from = selection.from_offsets_excluding.offset(stream_id);
        let local = self.is_local(stream_id);

        if !only_local || local {
            Some(StreamEventSelection {
                stream_id,
                from_exclusive: from,
                to_inclusive: OffsetOrMin::MAX,
                tag_subscriptions: selection.tag_subscriptions.as_tag_sets(local),
            })
        } else {
            None
        }
    }

    fn offsets(&self) -> BoxStream<'static, OffsetsResponse>;

    fn present(&self) -> BoxFuture<'static, OffsetMapOrMax> {
        self.offsets()
            .into_future()
            .map(move |(offsets, _)| OffsetMapOrMax::from(offsets.unwrap_or_default().present))
            .boxed()
    }

    fn persist(&self, events: Vec<(TagSet, Payload)>) -> BoxFuture<'static, anyhow::Result<Vec<PersistenceMeta>>>;

    fn bounded_forward(&self, selection: EventSelection) -> EventStreamOrError {
        let this = self.clone();
        self.present()
            .then(move |present| {
                future::try_join_all(
                    this.bounded_streams(selection, present)
                        .into_iter()
                        .map(|stream_selection| this.forward_stream(stream_selection)),
                )
                .map_ok(|event_streams| MergeOrdered::new_fixed(event_streams).boxed())
                .boxed()
            })
            .boxed()
    }

    fn bounded_forward_per_stream(&self, selection: EventSelection) -> EventStreamOrError {
        let this = self.clone();
        self.present()
            .then(move |present| {
                future::try_join_all(
                    this.bounded_streams(selection, present)
                        .into_iter()
                        .map(|stream_selection| this.forward_stream(stream_selection)),
                )
                .map_ok(|event_streams| stream::iter(event_streams).flatten().boxed())
                .boxed()
            })
            .boxed()
    }

    fn bounded_backward(&self, selection: EventSelection) -> EventStreamOrError {
        let this = self.clone();
        self.present()
            .then(move |present| {
                future::try_join_all(
                    this.bounded_streams(selection, present)
                        .into_iter()
                        .map(|stream_selection| this.backward_stream(stream_selection)),
                )
                .map_ok(|event_streams| MergeOrdered::new_fixed(event_streams).map(|reverse| reverse.0).boxed())
                .boxed()
            })
            .boxed()
    }

    fn unbounded_forward_per_stream(&self, selection: EventSelection) -> BoxStream<'static, Event<Payload>> {
        debug_assert!(selection.to_offsets_including.get_default() == OffsetOrMin::MAX);
        let this = self.clone();
        let this2 = self.clone();
        self.known_streams()
            .filter_map(move |stream_id| future::ready(this.unbounded_stream(&selection, stream_id)))
            .then(move |stream_selection| this2.forward_stream(stream_selection))
            .map(|res| res.unwrap())
            .merge_unordered()
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use actyxos_sdk::{service::Order, tags, OffsetOrMin, StreamId};
    use ax_futures_util::stream::Drainer;
    use num_traits::Bounded;
    use trees::{OffsetMapOrMax, TagSubscription, TagSubscriptions};

    use super::*;
    use crate::BanyanStore;

    pub async fn await_stream_offsets(store: &BanyanStore, other_stores: &[&BanyanStore], offsets: &OffsetMapOrMax) {
        for other in other_stores {
            store
                .ipfs()
                .add_address(&other.ipfs().local_peer_id(), other.ipfs().listeners()[0].clone());
        }

        let mut waiting_for: BTreeMap<_, _> = offsets
            .streams()
            .map(|stream_id| (stream_id, offsets.offset(stream_id)))
            .collect();
        let mut present = store.offsets().map(|o| o.present);
        while let Some(incoming) = present.next().await {
            // waiting_for.retain(|stream_id, offset| offset < incoming.offset(stream_id.clone())); // "1.53.0"
            incoming.streams().for_each(|stream_id| {
                if let Some(o) = waiting_for.get(&stream_id) {
                    if o <= &incoming.offset(stream_id) {
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
        stream: BoxStream<'static, Event<Payload>>,
        selection: &EventSelection,
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
        let store = BanyanStore::test("swarm_test").await.unwrap();
        let stream_id = store.node_id().stream(0.into());

        store.persist(vec![(tags!(), Payload::empty())]).await.unwrap();

        let mut stream = Drainer::new(
            store
                .forward_stream(StreamEventSelection {
                    stream_id,
                    from_exclusive: OffsetOrMin::MIN,
                    to_inclusive: OffsetOrMin::ZERO,
                    tag_subscriptions: vec![tags!()],
                })
                .await
                .unwrap(),
        );
        let res = stream.next().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(stream.next(), None);

        let mut stream = Drainer::new(
            store
                .forward_stream(StreamEventSelection {
                    stream_id,
                    from_exclusive: OffsetOrMin::MIN,
                    to_inclusive: OffsetOrMin::ZERO,
                    tag_subscriptions: vec![tags!("nothing")],
                })
                .await
                .unwrap(),
        );
        assert_eq!(stream.next(), None);

        let mut stream = Drainer::new(
            store
                .forward_stream(StreamEventSelection {
                    stream_id,
                    from_exclusive: OffsetOrMin::MIN,
                    to_inclusive: OffsetOrMin::ZERO,
                    tag_subscriptions: vec![tags!()],
                })
                .await
                .unwrap(),
        );
        let res = stream.next().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(stream.next(), None); // bounded -> complete

        let mut stream = Drainer::new(
            store
                .forward_stream(StreamEventSelection {
                    stream_id,
                    from_exclusive: OffsetOrMin::MIN,
                    to_inclusive: OffsetOrMin::MAX,
                    tag_subscriptions: vec![tags!()],
                })
                .await
                .unwrap(),
        );
        let res = stream.next().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(stream.next(), Some(vec![])); // unbounded -> keep running
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_backward_stream() {
        let store = BanyanStore::test("swarm_test").await.unwrap();
        let stream_id = store.node_id().stream(0.into());

        store.persist(vec![(tags!(), Payload::empty())]).await.unwrap();

        let mut stream = Drainer::new(
            store
                .backward_stream(StreamEventSelection {
                    stream_id,
                    from_exclusive: OffsetOrMin::MIN,
                    to_inclusive: OffsetOrMin::ZERO,
                    tag_subscriptions: vec![tags!()],
                })
                .await
                .unwrap(),
        );
        let res = stream.next().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(stream.next(), None);

        let mut stream = Drainer::new(
            store
                .backward_stream(StreamEventSelection {
                    stream_id,
                    from_exclusive: OffsetOrMin::MIN,
                    to_inclusive: OffsetOrMin::ZERO,
                    tag_subscriptions: vec![tags!("nothing")],
                })
                .await
                .unwrap(),
        );
        assert_eq!(stream.next(), None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_bounded_forward() {
        let store1 = BanyanStore::test("swarm_test1").await.unwrap();
        let store2 = BanyanStore::test("swarm_test2").await.unwrap();

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

        let ranges = [
            (stream_id1, OffsetOrMin::MIN, 2u32.into()),
            (stream_id2, OffsetOrMin::MIN, 2u32.into()),
        ];

        fn assert_forward_completed(
            stream: BoxStream<'static, Event<Payload>>,
            selection: &EventSelection,
            len: usize,
        ) {
            assert_stream(stream, selection, len, Order::Asc, true);
        }

        let sel_all = EventSelection::create("FROM 'test'", &ranges).unwrap();
        let _ = await_stream_offsets(&store1, &[&store2], &sel_all.to_offsets_including).await;

        assert_forward_completed(store1.bounded_forward(sel_all.clone()).await.unwrap(), &sel_all, 6);

        let sel_all_max = EventSelection::create(
            "FROM 'test'",
            &[
                (stream_id1, OffsetOrMin::MIN, OffsetOrMin::MAX),
                (stream_id2, OffsetOrMin::MIN, OffsetOrMin::MAX),
            ],
        )
        .unwrap();
        assert_forward_completed(
            store1.bounded_forward(sel_all_max.clone()).await.unwrap(),
            &sel_all_max,
            6,
        );

        // stream1
        let sel_expr_local = EventSelection::create("FROM isLocal", &ranges).unwrap();
        assert_forward_completed(
            store1.bounded_forward(sel_expr_local.clone()).await.unwrap(),
            &sel_expr_local,
            3,
        );

        let sel_stream1 =
            EventSelection::create("FROM 'test'", &[(stream_id1, OffsetOrMin::MIN, 2u32.into())]).unwrap();
        assert_forward_completed(
            store1.bounded_forward(sel_stream1.clone()).await.unwrap(),
            &sel_stream1,
            3,
        );

        let sel_stream1_single =
            EventSelection::create("FROM 'test'", &[(stream_id1, OffsetOrMin::ZERO, 1u32.into())]).unwrap();
        assert_forward_completed(
            store1.bounded_forward(sel_stream1_single.clone()).await.unwrap(),
            &sel_stream1_single,
            1,
        );

        // stream2
        let sel_expr_stream2 = EventSelection::create("FROM 'test:stream2'", &ranges).unwrap();
        assert_forward_completed(
            store1.bounded_forward(sel_expr_stream2.clone()).await.unwrap(),
            &sel_expr_stream2,
            3,
        );

        let sel_stream_2 =
            EventSelection::create("FROM 'test'", &[(stream_id2, OffsetOrMin::MIN, 2u32.into())]).unwrap();
        assert_forward_completed(
            store1.bounded_forward(sel_stream_2.clone()).await.unwrap(),
            &sel_stream_2,
            3,
        );

        let sel_stream2_single =
            EventSelection::create("FROM 'test'", &[(stream_id2, OffsetOrMin::ZERO, 1u32.into())]).unwrap();
        assert_forward_completed(
            store1.bounded_forward(sel_stream2_single.clone()).await.unwrap(),
            &sel_stream2_single,
            1,
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_bounded_backward() {
        let store1 = BanyanStore::test("swarm_test1").await.unwrap();
        let store2 = BanyanStore::test("swarm_test2").await.unwrap();

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

        let ranges = [
            (stream_id1, OffsetOrMin::MIN, 2u32.into()),
            (stream_id2, OffsetOrMin::MIN, 2u32.into()),
        ];

        fn assert_backward_completed(
            stream: BoxStream<'static, Event<Payload>>,
            selection: &EventSelection,
            len: usize,
        ) {
            assert_stream(stream, selection, len, Order::Desc, true);
        }

        let sel_all = EventSelection::create("FROM 'test'", &ranges).unwrap();
        let _ = await_stream_offsets(&store1, &[&store2], &sel_all.to_offsets_including).await;

        assert_backward_completed(store1.bounded_backward(sel_all.clone()).await.unwrap(), &sel_all, 6);

        let sel_all_max = EventSelection::create(
            "FROM 'test'",
            &[
                (stream_id1, OffsetOrMin::MIN, OffsetOrMin::MAX),
                (stream_id2, OffsetOrMin::MIN, OffsetOrMin::MAX),
            ],
        )
        .unwrap();
        assert_backward_completed(
            store1.bounded_backward(sel_all_max.clone()).await.unwrap(),
            &sel_all_max,
            6,
        );

        // stream1
        let sel_expr_local = EventSelection::create("FROM isLocal", &ranges).unwrap();
        assert_backward_completed(
            store1.bounded_backward(sel_expr_local.clone()).await.unwrap(),
            &sel_expr_local,
            3,
        );

        let sel_stream1 =
            EventSelection::create("FROM 'test'", &[(stream_id1, OffsetOrMin::MIN, 2u32.into())]).unwrap();
        assert_backward_completed(
            store1.bounded_backward(sel_stream1.clone()).await.unwrap(),
            &sel_stream1,
            3,
        );

        let sel_stream1_single =
            EventSelection::create("FROM 'test'", &[(stream_id1, OffsetOrMin::ZERO, 1u32.into())]).unwrap();
        assert_backward_completed(
            store1.bounded_backward(sel_stream1_single.clone()).await.unwrap(),
            &sel_stream1_single,
            1,
        );

        // stream2
        let sel_expr_stream2 = EventSelection::create("FROM 'test:stream2'", &ranges).unwrap();
        assert_backward_completed(
            store1.bounded_backward(sel_expr_stream2.clone()).await.unwrap(),
            &sel_expr_stream2,
            3,
        );

        let sel_stream_2 =
            EventSelection::create("FROM 'test'", &[(stream_id2, OffsetOrMin::MIN, 2u32.into())]).unwrap();
        assert_backward_completed(
            store1.bounded_backward(sel_stream_2.clone()).await.unwrap(),
            &sel_stream_2,
            3,
        );

        let sel_stream2_single =
            EventSelection::create("FROM 'test'", &[(stream_id2, OffsetOrMin::ZERO, 1u32.into())]).unwrap();
        assert_backward_completed(
            store1.bounded_backward(sel_stream2_single.clone()).await.unwrap(),
            &sel_stream2_single,
            1,
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_unbounded_forward() {
        let store1 = BanyanStore::test("swarm_test1").await.unwrap();
        let store2 = BanyanStore::test("swarm_test2").await.unwrap();

        let stream_id1 = store1.node_id().stream(0.into());
        let stream_id2 = store2.node_id().stream(0.into());

        store1
            .persist(vec![(tags!("test:unbounded:forward"), Payload::empty())])
            .await
            .unwrap();

        fn assert_forward_not_completed(
            stream: BoxStream<'static, Event<Payload>>,
            selection: &EventSelection,
            len: usize,
        ) {
            assert_stream(stream, selection, len, Order::StreamAsc, false);
        }

        fn after(offsets: &[(StreamId, OffsetOrMin)]) -> EventSelection {
            // otherwise we get events from other tests
            let tag_subscriptions = TagSubscriptions::new(vec![TagSubscription::new(tags!("test:unbounded:forward"))]);
            EventSelection {
                tag_subscriptions,
                from_offsets_excluding: OffsetMapOrMax::from_entries(offsets),
                to_offsets_including: OffsetMapOrMax::max_value(),
            }
        }

        let store1_clone = store1.clone();
        let store2_clone = store2.clone();
        let handle = tokio::spawn(async move {
            let store_rx = BanyanStore::test("swarm_test_rx").await.unwrap();
            let offsets = [(stream_id1, OffsetOrMin::ZERO)];
            let target = [(stream_id1, 1u32.into()), (stream_id2, 0u32.into())];
            let selection = after(&offsets);
            // stream1 is below range and stream2 non-existant at this point
            let stream = store_rx.unbounded_forward_per_stream(selection.clone());
            let _ = await_stream_offsets(
                &store_rx,
                &[&store1_clone, &store2_clone],
                &OffsetMapOrMax::from_entries(&target),
            )
            .await;
            assert_forward_not_completed(stream, &selection, 2);

            // let unknown = after(&[(non_existant(), OffsetOrMin::MIN)]);
            // assert!(matches!(
            //     store_rx.unbounded_forward_per_stream(&unknown),
            //     Err(EventStoreError::UnknownStream(_))
            // ));
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
}
