use std::cmp::Reverse;

use actyxos_sdk::{
    service::OffsetsResponse, Event, LamportTimestamp, Offset, OffsetMap, OffsetOrMin, Payload, StreamId, StreamNr,
    TagSet, Timestamp,
};
use ax_futures_util::{prelude::AxStreamExt, stream::MergeOrdered};
use derive_more::Display;
use futures::{
    future::{self, BoxFuture},
    stream::{self, BoxStream},
    FutureExt, StreamExt, TryFutureExt,
};
use trees::{OffsetMapOrMax, TagSubscriptions};

use crate::selection::StreamEventSelection;

#[derive(Clone, Debug, Display)]
pub enum EventStoreError {
    InvalidUpperBounds,
}
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

    fn bounded_streams(
        &self,
        tag_subscriptions: TagSubscriptions,
        from_offsets_excluding: Option<OffsetMap>,
        to_offsets_including: OffsetMap,
    ) -> BoxFuture<'static, Result<Vec<StreamEventSelection>, EventStoreError>> {
        let only_local = tag_subscriptions.only_local();
        let this = self.clone();
        self.present()
            .then(move |present| {
                if present.union(&to_offsets_including) != present {
                    return future::err(EventStoreError::InvalidUpperBounds).boxed();
                }
                let from_or_min = from_offsets_excluding.map(OffsetMapOrMax::from).unwrap_or_default();
                let res: Vec<_> = to_offsets_including
                    .streams()
                    .filter_map(|stream_id| {
                        let from = from_or_min.offset(stream_id);
                        let to = to_offsets_including.offset(stream_id).min(present.offset(stream_id));
                        let local = this.is_local(stream_id);

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
                future::ok(res).boxed()
            })
            .boxed()
    }

    fn unbounded_stream(
        &self,
        stream_id: StreamId,
        tag_subscriptions: &TagSubscriptions,
        from_exclusive: OffsetOrMin,
    ) -> Option<StreamEventSelection> {
        let only_local = tag_subscriptions.only_local();
        let local = self.is_local(stream_id);
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

    fn offsets(&self) -> BoxStream<'static, OffsetsResponse>;

    fn present(&self) -> BoxFuture<'static, OffsetMap> {
        self.offsets()
            .into_future()
            .map(move |(offsets, _)| offsets.unwrap_or_default().present)
            .boxed()
    }

    fn persist(&self, events: Vec<(TagSet, Payload)>) -> BoxFuture<'static, anyhow::Result<Vec<PersistenceMeta>>>;

    fn bounded_forward(
        &self,
        tag_subscriptions: TagSubscriptions,
        from_offsets_excluding: Option<OffsetMap>,
        to_offsets_including: OffsetMap,
    ) -> EventStreamOrError {
        let this = self.clone();
        self.bounded_streams(tag_subscriptions, from_offsets_excluding, to_offsets_including)
            .and_then(move |stream_selections| {
                future::try_join_all(
                    stream_selections
                        .into_iter()
                        .map(|stream_selection| this.forward_stream(stream_selection)),
                )
                .map_ok(|event_streams| MergeOrdered::new_fixed(event_streams).boxed())
                .boxed()
            })
            .boxed()
    }

    fn bounded_forward_per_stream(
        &self,
        tag_subscriptions: TagSubscriptions,
        from_offsets_excluding: Option<OffsetMap>,
        to_offsets_including: OffsetMap,
    ) -> EventStreamOrError {
        let this = self.clone();
        self.bounded_streams(tag_subscriptions, from_offsets_excluding, to_offsets_including)
            .and_then(move |stream_selections| {
                future::try_join_all(
                    stream_selections
                        .into_iter()
                        .map(|stream_selection| this.forward_stream(stream_selection)),
                )
                .map_ok(|event_streams| stream::iter(event_streams).flatten().boxed())
                .boxed()
            })
            .boxed()
    }

    fn bounded_backward(
        &self,
        tag_subscriptions: TagSubscriptions,
        from_offsets_excluding: Option<OffsetMap>,
        to_offsets_including: OffsetMap,
    ) -> EventStreamOrError {
        let this = self.clone();
        self.bounded_streams(tag_subscriptions, from_offsets_excluding, to_offsets_including)
            .and_then(move |stream_selections| {
                future::try_join_all(
                    stream_selections
                        .into_iter()
                        .map(|stream_selection| this.backward_stream(stream_selection)),
                )
                .map_ok(|event_streams| MergeOrdered::new_fixed(event_streams).map(|reverse| reverse.0).boxed())
                .boxed()
            })
            .boxed()
    }

    fn unbounded_forward_per_stream(
        &self,
        tag_subscriptions: TagSubscriptions,
        from_offsets_excluding: Option<OffsetMap>,
    ) -> BoxStream<'static, Event<Payload>> {
        let this = self.clone();
        let this2 = self.clone();
        let from_or_min = from_offsets_excluding.map(OffsetMapOrMax::from).unwrap_or_default();
        self.known_streams()
            .filter_map(move |stream_id| {
                future::ready(this.unbounded_stream(stream_id, &tag_subscriptions, from_or_min.offset(stream_id)))
            })
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
    use crate::{selection::EventSelection, BanyanStore};

    fn offset_map(entries: &[(StreamId, u32)]) -> BTreeMap<StreamId, Offset> {
        entries
            .iter()
            .copied()
            .map(|(stream_id, offset)| (stream_id, offset.into()))
            .collect()
    }

    async fn await_stream_offsets<'a>(
        store: &'a BanyanStore,
        other_stores: &'a [&BanyanStore],
        offsets: &'a [(StreamId, u32)],
    ) {
        for other in other_stores {
            store
                .ipfs()
                .add_address(&other.ipfs().local_peer_id(), other.ipfs().listeners()[0].clone());
        }

        let mut waiting_for = offset_map(offsets);
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
        expr: &'static str,
        from_offsets_excluding: OffsetMapOrMax,
        to_offsets_including: OffsetMapOrMax,
        len: usize,
        order: Order,
        completed: bool,
    ) {
        let query = &expr.parse::<actyxos_sdk::language::Query>().unwrap();
        let tag_subscriptions = query.into();
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
    async fn test_bounded() {
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

        async fn assert_bounded<'a>(
            store: &'a BanyanStore,
            expr: &'static str,
            from: Option<&'a [(StreamId, u32)]>,
            to: &'a [(StreamId, u32)],
            len: usize,
        ) {
            let from: Option<OffsetMap> = from.map(offset_map).map(Into::into);
            let to: OffsetMap = offset_map(to).into();
            let query = &expr.parse::<actyxos_sdk::language::Query>().unwrap();
            let tag_subscriptions: TagSubscriptions = query.into();

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

        let max = [(stream_id1, 2), (stream_id2, 2u32)];
        let _ = await_stream_offsets(&store1, &[&store2], &max).await;

        // all
        assert_bounded(&store1, "FROM 'test'", None, &max, 6).await;

        // stream1
        assert_bounded(&store1, "FROM isLocal", None, &max, 3).await;
        assert_bounded(&store1, "FROM 'test'", None, &[(stream_id1, 2)], 3).await;
        assert_bounded(&store1, "FROM 'test'", Some(&[(stream_id1, 0)]), &[(stream_id1, 1)], 1).await;

        // stream2
        assert_bounded(&store1, "FROM 'test:stream2'", None, &max, 3).await;
        assert_bounded(&store1, "FROM 'test'", None, &[(stream_id2, 2)], 3).await;
        assert_bounded(&store1, "FROM 'test'", Some(&[(stream_id2, 0)]), &[(stream_id2, 1)], 1).await;

        // fixme exceed upper bounds
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

        async fn assert_unbounded<'a>(
            stream: BoxStream<'static, Event<Payload>>,
            expr: &'static str,
            from: Option<&'a [(StreamId, u32)]>,
            len: usize,
        ) {
            let from: OffsetMap = from.map(|entries| offset_map(entries).into()).unwrap_or_default();

            assert_stream(
                stream,
                expr,
                from.into(),
                OffsetMapOrMax::max_value(),
                len,
                Order::StreamAsc,
                false,
            );
        }

        let store1_clone = store1.clone();
        let store2_clone = store2.clone();
        let handle = tokio::spawn(async move {
            let store_rx = BanyanStore::test("swarm_test_rx").await.unwrap();
            let tag_subscriptions = TagSubscriptions::new(vec![TagSubscription::new(tags!("test:unbounded:forward"))]);
            // stream1 is below range and stream2 non-existant at this point
            // let selection = EventSelection::after(tag_subscriptions, from.clone().into());
            let from = [(stream_id1, 0)];
            let stream = store_rx.unbounded_forward_per_stream(tag_subscriptions, Some(offset_map(&from).into()));
            let _ = await_stream_offsets(
                &store_rx,
                &[&store1_clone, &store2_clone],
                &[(stream_id1, 1), (stream_id2, 0)],
            )
            .await;
            assert_unbounded(stream, "FROM 'test:unbounded:forward'", Some(&from), 2).await;

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
