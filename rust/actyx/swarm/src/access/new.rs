use std::cmp::Reverse;

use actyxos_sdk::{Event, OffsetOrMin, Payload};
use ax_futures_util::{prelude::AxStreamExt, stream::MergeOrdered};
use futures::{
    future::{self, BoxFuture},
    stream::{self, BoxStream},
    FutureExt, StreamExt, TryFutureExt,
};

use super::{
    ConsumerAccessError, EventOrHeartbeat, EventSelection, EventStoreConsumerAccess, EventStreamOrError,
    StreamEventSelection,
};

fn mk_forward_stream(store: &impl EventStoreConsumerAccess) -> impl FnMut(StreamEventSelection) -> EventStreamOrError {
    let store = store.clone();
    move |stream_selection| {
        assert!(stream_selection.from_exclusive < stream_selection.to_inclusive);
        store
            .stream_forward(stream_selection, true)
            .map_ok(move |stream| {
                stream
                    // FIXME remove heartbeats
                    .filter_map(|eoh| {
                        future::ready(match eoh {
                            EventOrHeartbeat::Event(event) => Some(event),
                            _ => None,
                        })
                    })
                    .boxed()
            })
            .boxed()
    }
}

fn mk_backward_stream(
    store: &impl EventStoreConsumerAccess,
) -> impl FnMut(
    StreamEventSelection,
) -> BoxFuture<'static, Result<BoxStream<'static, Reverse<Event<Payload>>>, ConsumerAccessError>> {
    let store = store.clone();
    move |stream_selection| {
        assert!(stream_selection.from_exclusive < stream_selection.to_inclusive);
        store
            .stream_backward(stream_selection)
            .map_ok(move |stream| stream.map(Reverse).boxed())
            .boxed()
    }
}

pub fn bounded_forward(store: &impl EventStoreConsumerAccess, selection: &EventSelection) -> EventStreamOrError {
    // TODO: assert selection.to_offsets_including =< store.present?
    future::try_join_all(selection.bounded_streams(store).map(mk_forward_stream(store)))
        .map_ok(|event_streams| MergeOrdered::new_fixed(event_streams).boxed())
        .boxed()
}

pub fn bounded_forward_per_stream(
    store: &impl EventStoreConsumerAccess,
    selection: &EventSelection,
) -> EventStreamOrError {
    // FIXME tests
    future::try_join_all(selection.bounded_streams(store).map(mk_forward_stream(store)))
        .map_ok(|event_streams| stream::iter(event_streams).flatten().boxed())
        .boxed()
}

pub fn bounded_backward(store: &impl EventStoreConsumerAccess, selection: &EventSelection) -> EventStreamOrError {
    future::try_join_all(selection.bounded_streams(store).map(mk_backward_stream(store)))
        .map_ok(|event_streams| MergeOrdered::new_fixed(event_streams).map(|reverse| reverse.0).boxed())
        .boxed()
}

pub fn unbounded_forward_per_stream(
    store: &impl EventStoreConsumerAccess,
    selection: &EventSelection,
) -> BoxStream<'static, Event<Payload>> {
    debug_assert!(selection.to_offsets_including.get_default() == OffsetOrMin::MAX);
    let store = store.clone();
    let store2 = store.clone();
    let selection = selection.clone();
    store
        .stream_known_streams()
        .filter_map(move |stream_id| future::ready(selection.unbounded_stream(&store, stream_id)))
        .then(move |stream_selection| mk_forward_stream(&store2)(stream_selection))
        .map(|res| res.unwrap())
        .merge_unordered()
        .boxed()
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, convert::TryFrom};

    use actyxos_sdk::{service::Order, tags, NodeId, OffsetOrMin, StreamId};
    use ax_futures_util::stream::Drainer;
    use num_traits::Bounded;
    use trees::{OffsetMapOrMax, TagSubscription, TagSubscriptions};

    use super::*;
    use crate::{access::EventSelection, BanyanStore, EventStore, Present};

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
    async fn test_mk_forward_stream() {
        let store = BanyanStore::test("swarm_test").await.unwrap();
        let stream_id = store.node_id().stream(0.into());

        store.persist(vec![(tags!(), Payload::empty())]).await.unwrap();

        let mut stream = Drainer::new(
            mk_forward_stream(&store)(StreamEventSelection {
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
            mk_forward_stream(&store)(StreamEventSelection {
                stream_id,
                from_exclusive: OffsetOrMin::MIN,
                to_inclusive: OffsetOrMin::ZERO,
                tag_subscriptions: vec![tags!("nothing")],
            })
            .await
            .unwrap(),
        );
        assert_eq!(stream.next(), None);

        // let mut stream = Drainer::new(
        //     mk_forward_stream(&store, true)(StreamEventSelection {
        //         stream_id,
        //         from_exclusive: OffsetOrMin::MIN,
        //         to_inclusive: OffsetOrMin::MAX,
        //         tag_subscriptions: vec![tags!()],
        //     })
        //     .await
        //     .unwrap(),
        // );
        // let res = stream.next().unwrap();
        // assert_eq!(res.len(), 1);
        // assert_eq!(stream.next(), None); // bounded -> complete ??

        let mut stream = Drainer::new(
            mk_forward_stream(&store)(StreamEventSelection {
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
    async fn test_mk_backward_stream() {
        let store = BanyanStore::test("swarm_test").await.unwrap();
        let stream_id = store.node_id().stream(0.into());

        store.persist(vec![(tags!(), Payload::empty())]).await.unwrap();

        let mut stream = Drainer::new(
            mk_backward_stream(&store)(StreamEventSelection {
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
            mk_backward_stream(&store)(StreamEventSelection {
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

        assert_forward_completed(bounded_forward(&store1, &sel_all).await.unwrap(), &sel_all, 6);

        // stream1
        let sel_expr_local = EventSelection::create("FROM isLocal", &ranges).unwrap();
        assert_forward_completed(
            bounded_forward(&store1, &sel_expr_local).await.unwrap(),
            &sel_expr_local,
            3,
        );

        let sel_stream1 =
            EventSelection::create("FROM 'test'", &[(stream_id1, OffsetOrMin::MIN, 2u32.into())]).unwrap();
        assert_forward_completed(bounded_forward(&store1, &sel_stream1).await.unwrap(), &sel_stream1, 3);

        let sel_stream1_single =
            EventSelection::create("FROM 'test'", &[(stream_id1, OffsetOrMin::ZERO, 1u32.into())]).unwrap();
        assert_forward_completed(
            bounded_forward(&store1, &sel_stream1_single).await.unwrap(),
            &sel_stream1_single,
            1,
        );

        // stream2
        let sel_expr_stream2 = EventSelection::create("FROM 'test:stream2'", &ranges).unwrap();
        assert_forward_completed(
            bounded_forward(&store1, &sel_expr_stream2).await.unwrap(),
            &sel_expr_stream2,
            3,
        );

        let sel_stream_2 =
            EventSelection::create("FROM 'test'", &[(stream_id2, OffsetOrMin::MIN, 2u32.into())]).unwrap();
        assert_forward_completed(bounded_forward(&store1, &sel_stream_2).await.unwrap(), &sel_stream_2, 3);

        let sel_stream2_single =
            EventSelection::create("FROM 'test'", &[(stream_id2, OffsetOrMin::ZERO, 1u32.into())]).unwrap();
        assert_forward_completed(
            bounded_forward(&store1, &sel_stream2_single).await.unwrap(),
            &sel_stream2_single,
            1,
        );

        let unknown = EventSelection::create(
            "FROM 'test'",
            &[(
                NodeId::try_from("F71z5SVr2cJ.GYu3WGYZAg1pR7R7LAn6dXuwB3CU9V.")
                    .unwrap()
                    .stream(0.into()),
                OffsetOrMin::ZERO,
                1u32.into(),
            )],
        )
        .unwrap();
        assert!(matches!(
            bounded_forward(&store1, &unknown).await,
            Err(ConsumerAccessError::UnknownStream(_))
        ));
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

        assert_backward_completed(bounded_backward(&store1, &sel_all).await.unwrap(), &sel_all, 6);

        // stream1
        let sel_expr_local = EventSelection::create("FROM isLocal", &ranges).unwrap();
        assert_backward_completed(
            bounded_backward(&store1, &sel_expr_local).await.unwrap(),
            &sel_expr_local,
            3,
        );

        let sel_stream1 =
            EventSelection::create("FROM 'test'", &[(stream_id1, OffsetOrMin::MIN, 2u32.into())]).unwrap();
        assert_backward_completed(bounded_backward(&store1, &sel_stream1).await.unwrap(), &sel_stream1, 3);

        let sel_stream1_single =
            EventSelection::create("FROM 'test'", &[(stream_id1, OffsetOrMin::ZERO, 1u32.into())]).unwrap();
        assert_backward_completed(
            bounded_backward(&store1, &sel_stream1_single).await.unwrap(),
            &sel_stream1_single,
            1,
        );

        // stream2
        let sel_expr_stream2 = EventSelection::create("FROM 'test:stream2'", &ranges).unwrap();
        assert_backward_completed(
            bounded_backward(&store1, &sel_expr_stream2).await.unwrap(),
            &sel_expr_stream2,
            3,
        );

        let sel_stream_2 =
            EventSelection::create("FROM 'test'", &[(stream_id2, OffsetOrMin::MIN, 2u32.into())]).unwrap();
        assert_backward_completed(
            bounded_backward(&store1, &sel_stream_2).await.unwrap(),
            &sel_stream_2,
            3,
        );

        let sel_stream2_single =
            EventSelection::create("FROM 'test'", &[(stream_id2, OffsetOrMin::ZERO, 1u32.into())]).unwrap();
        assert_backward_completed(
            bounded_backward(&store1, &sel_stream2_single).await.unwrap(),
            &sel_stream2_single,
            1,
        );

        // FIXME: always assert exists inside ESCA::stream_{forward|backward}()
        // let unknown = EventSelection::create(
        //     "FROM 'test'",
        //     &[(
        //         NodeId::try_from("F71z5SVr2cJ.GYu3WGYZAg1pR7R7LAn6dXuwB3CU9V.")
        //             .unwrap()
        //             .stream(0.into()),
        //         OffsetOrMin::MIN,
        //         OffsetOrMin::ZERO,
        //     )],
        // )
        // .unwrap();
        // assert!(matches!(
        //     bounded_backward(&store1, &unknown).await,
        //     Err(ConsumerAccessError::UnknownStream(_))
        // ));
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
            let stream = unbounded_forward_per_stream(&store_rx, &selection);
            let _ = await_stream_offsets(
                &store_rx,
                &[&store1_clone, &store2_clone],
                &OffsetMapOrMax::from_entries(&target),
            )
            .await;
            assert_forward_not_completed(stream, &selection, 2);
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
