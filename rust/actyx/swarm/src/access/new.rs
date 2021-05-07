use std::cmp::Reverse;

use actyxos_sdk::{Event, OffsetOrMin, Payload};
use ax_futures_util::stream::MergeOrdered;
use futures::{
    future::{self, BoxFuture},
    stream::BoxStream,
    FutureExt, StreamExt, TryFutureExt,
};

use super::{
    ConsumerAccessError, EventOrHeartbeat, EventSelection, EventStoreConsumerAccess, EventStreamOrError,
    StreamEventSelection,
};

fn mk_forward_stream(store: &impl EventStoreConsumerAccess) -> impl FnMut(StreamEventSelection) -> EventStreamOrError {
    let store = store.clone();
    move |stream_selection| {
        store
            .stream_forward(stream_selection, true)
            .map_ok(move |stream| {
                stream
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
        store
            .stream_backward(stream_selection)
            .map_ok(move |stream| stream.map(Reverse).boxed())
            .boxed()
    }
}

pub fn bounded_forward(store: &impl EventStoreConsumerAccess, selection: EventSelection) -> EventStreamOrError {
    future::try_join_all(selection.bounded_streams(store).map(mk_forward_stream(store)))
        .map_ok(|event_streams| MergeOrdered::new_fixed(event_streams).boxed())
        .boxed()
}

pub fn bounded_forward_per_stream(
    store: &impl EventStoreConsumerAccess,
    selection: EventSelection,
) -> EventStreamOrError {
    future::try_join_all(selection.bounded_streams(store).map(mk_forward_stream(store)))
        .map_ok(|event_streams| futures::stream::iter(event_streams).flatten().boxed())
        .boxed()
}
pub fn bounded_backward(store: &impl EventStoreConsumerAccess, selection: EventSelection) -> EventStreamOrError {
    future::try_join_all(selection.bounded_streams(store).map(mk_backward_stream(store)))
        .map_ok(|event_streams| MergeOrdered::new_fixed(event_streams).map(|reverse| reverse.0).boxed())
        .boxed()
}

pub fn unbounded_forward(
    store: &impl EventStoreConsumerAccess,
    selection: EventSelection,
) -> BoxStream<'static, Event<Payload>> {
    assert!(selection.to_offsets_including.get_default() == OffsetOrMin::MAX);
    let store = store.clone();
    let store2 = store.clone();
    store
        .stream_known_streams()
        .filter_map(move |stream_id| future::ready(selection.unbounded_stream(&store, stream_id)))
        .then(mk_forward_stream(&store2))
        .map(|res| res.unwrap())
        .flatten()
        .boxed()
}

#[cfg(test)]
mod tests {
    use actyxos_sdk::{service::Order, tags, OffsetOrMin};

    use super::*;
    use crate::{
        access::{common::*, tests::*, EventSelection},
        BanyanStore, EventStore,
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn should_deliver_bounded_stream() {
        let store1 = BanyanStore::test("swarm_test1").await.unwrap();
        let store2 = BanyanStore::test("swarm_test2").await.unwrap();

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

        let stream_id1 = store1.node_id().stream(0.into());
        let stream_id2 = store2.node_id().stream(0.into());

        await_known_streams(&store1, &[stream_id1, stream_id2]).await;

        let ranges = [
            (stream_id1, OffsetOrMin::MIN, 2u32.into()),
            (stream_id2, OffsetOrMin::MIN, 2u32.into()),
        ];
        let selection = EventSelection::create("('stream:1' | 'stream:2') & 'keep'", &ranges).unwrap();
        let stream = bounded_backward(&store1, selection.clone()).await.unwrap();

        let expected = num_streams * 6 / 2; // 6 events per stream inside the range, half of them filtered out
        let completed = true;
        assert_event_stream_matches(stream, expected, &selection, Some(Order::Asc), completed);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn should_deliver_live_stream() {
        let store1 = BanyanStore::test("swarm_test1").await.unwrap();
        let store2 = BanyanStore::test("swarm_test2").await.unwrap();
        let store3 = BanyanStore::test("swarm_test3").await.unwrap();

        let num_events = 4u32;
        let num_streams = 3;

        let event_stream_ids: Vec<_> = [&store1, &store2, &store3]
            .iter()
            .map(|store| store.node_id().stream(0.into()))
            .collect();

        // batch 1
        let _ = store1.persist(mk_events(num_events, 1)).await.unwrap();
        let _ = store2.persist(mk_events(num_events, 2)).await.unwrap();
        let _ = store3.persist(mk_events(num_events, 3)).await.unwrap();

        let handle = tokio::spawn(async move {
            let store_rx = BanyanStore::test("swarm_test_rx").await.unwrap();
            await_known_streams(&store_rx, &event_stream_ids).await;
            let ranges: Vec<_> = event_stream_ids
                .into_iter()
                .map(|stream_id| (stream_id, 1u32.into(), OffsetOrMin::MAX))
                .collect();
            let selection = EventSelection::create("('stream:1' | 'stream:2' | 'stream:3') & 'keep'", &ranges).unwrap();
            let stream = stream(&store_rx, selection.clone()).await.unwrap();

            // 1st batch: 1 event/stream (0,1 filtered by offset, 1+3 filtered by expr)
            // 2nd batch: 1 event/stream (none filtered out)
            let expected = num_streams * 2;
            let completed = false;
            assert_event_stream_matches(stream, expected, &selection, Some(Order::Asc), completed);
        });

        // batch 2
        let _ = store1.persist(mk_events(1, 1)).await.unwrap();
        let _ = store2.persist(mk_events(1, 2)).await.unwrap();
        let _ = store3.persist(mk_events(1, 3)).await.unwrap();

        // unblock everyone (FIXME: should this be necessary?)
        let _ = store1.persist(vec![(tags!("fini"), Payload::empty())]).await.unwrap();
        let _ = store2.persist(vec![(tags!("fini"), Payload::empty())]).await.unwrap();
        let _ = store3.persist(vec![(tags!("fini"), Payload::empty())]).await.unwrap();

        let _ = handle.await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn should_bring_in_new_streams() {
        let store_tx = BanyanStore::test("swarm_test1").await.unwrap();

        let event_stream_id = store_tx.node_id().stream(0.into());

        // batch 1
        let _ = store_tx.persist(mk_events(10, 1)).await.unwrap();

        let handle = tokio::spawn(async move {
            let store_rx = BanyanStore::test("swarm_test_rx").await.unwrap();
            await_known_streams(&store_rx, &[event_stream_id]).await;
            let ranges = vec![(event_stream_id, 9u32.into(), OffsetOrMin::MAX)];
            let selection = EventSelection::create("'stream:1' & 'keep'", &ranges).unwrap();
            let stream = stream(&store_rx, selection.clone()).await.unwrap();

            // 1st batch: 1 event/stream (0,1 filtered by offset, 1+3 filtered by expr)
            // 2nd batch: 1 event/stream (none filtered out)
            let expected = 1;
            let completed = false;
            assert_event_stream_matches(stream, expected, &selection, Some(Order::Asc), completed);
        });

        // batch 2
        let _ = store_tx.persist(mk_events(1, 1)).await.unwrap(); // first one that matches range + expr

        let _ = handle.await.unwrap();
    }
}
