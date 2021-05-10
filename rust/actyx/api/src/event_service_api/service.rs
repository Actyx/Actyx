use actyxos_sdk::{
    language,
    service::{
        self, EventResponse, NodeIdResponse, OffsetsResponse, Order, PublishEvent, PublishRequest, PublishResponse,
        PublishResponseKey, QueryRequest, QueryResponse, StartFrom, SubscribeMonotonicRequest,
        SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
    },
    Event, EventKey, Metadata, Payload,
};
use async_trait::async_trait;
use ax_futures_util::prelude::*;
use futures::{
    future,
    stream::{self, BoxStream, StreamExt},
    TryFutureExt,
};
use runtime::value::Value;
use swarm::access::EventSelection;
use swarm::{BanyanStore, EventStore, EventStoreError};
use thiserror::Error;
use trees::{OffsetMapOrMax, TagSubscriptions};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Store error: {0}")]
    StoreError(#[from] anyhow::Error),
    #[error("Access error: {0}")]
    EventStoreError(#[from] EventStoreError),
}

#[derive(Clone)]
pub struct EventService {
    store: BanyanStore,
}

impl EventService {
    pub fn new(store: BanyanStore) -> EventService {
        EventService { store }
    }
}

#[async_trait]
impl service::EventService for EventService {
    async fn node_id(&self) -> anyhow::Result<NodeIdResponse> {
        Ok(NodeIdResponse {
            node_id: self.store.node_id(),
        })
    }

    async fn offsets(&self) -> anyhow::Result<OffsetsResponse> {
        let response = self.store.offsets().next().await.unwrap_or_default();
        Ok(response)
    }

    async fn publish(&self, request: PublishRequest) -> anyhow::Result<PublishResponse> {
        let events = request
            .data
            .into_iter()
            .map(|PublishEvent { tags, payload }| (tags, payload))
            .collect();
        let response = self
            .store
            .persist(events)
            .map_ok(|keys| PublishResponse {
                data: keys
                    .into_iter()
                    .map(|(lamport, offset, stream_nr, timestamp)| PublishResponseKey {
                        lamport,
                        offset,
                        stream: self.store.node_id().stream(stream_nr),
                        timestamp,
                    })
                    .collect(),
            })
            .await
            .map_err(Error::StoreError)?;
        Ok(response)
    }

    async fn query(&self, request: QueryRequest) -> anyhow::Result<BoxStream<'static, QueryResponse>> {
        let from_offsets_excluding: OffsetMapOrMax = request.lower_bound.unwrap_or_default().into();
        let to_offsets_including: OffsetMapOrMax = request.upper_bound.into();
        let selection = EventSelection {
            tag_subscriptions: (&request.query).into(),
            from_offsets_excluding,
            to_offsets_including,
        };
        let response = match request.order {
            Order::Asc => self.store.bounded_forward(&selection),
            Order::Desc => self.store.bounded_backward(&selection),
            Order::StreamAsc => self.store.bounded_forward_per_stream(&selection),
        }
        .await
        .map_err(Error::EventStoreError)?
        .flat_map(mk_feed(request.query))
        .map(QueryResponse::Event);
        Ok(response.boxed())
    }

    async fn subscribe(&self, request: SubscribeRequest) -> anyhow::Result<BoxStream<'static, SubscribeResponse>> {
        let tag_subscriptions: TagSubscriptions = (&request.query).into();
        let present = OffsetMapOrMax::from(self.store.offsets().next().await.unwrap_or_default().present);

        let selection_bounded = EventSelection {
            tag_subscriptions: tag_subscriptions.clone(),
            from_offsets_excluding: OffsetMapOrMax::from(request.offsets.unwrap_or_default()),
            to_offsets_including: present.clone(),
        };
        let bounded = self
            .store
            .bounded_forward(&selection_bounded)
            .await
            .map_err(Error::EventStoreError)?;

        let selection_unbounded = EventSelection::after(tag_subscriptions, present);
        let unbounded = self.store.unbounded_forward_per_stream(&selection_unbounded);

        Ok(bounded
            .chain(unbounded)
            .flat_map(mk_feed(request.query))
            .map(SubscribeResponse::Event)
            .boxed())
    }

    async fn subscribe_monotonic(
        &self,
        request: SubscribeMonotonicRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let tag_subscriptions: TagSubscriptions = (&request.query).into();
        let present = OffsetMapOrMax::from(self.store.offsets().next().await.unwrap_or_default().present);

        let initial_latest = if let StartFrom::Offsets(offsets) = &request.from {
            let to_offsets_including = OffsetMapOrMax::from(offsets.clone());
            let selection = EventSelection::upto(tag_subscriptions.clone(), to_offsets_including);
            self.store
                .bounded_backward(&selection)
                .await
                .map_err(Error::EventStoreError)?
                .next()
                .await
                .map(|event| event.key)
                .unwrap_or_default()
        } else {
            EventKey::default()
        };

        let selection_bounded = EventSelection {
            tag_subscriptions: tag_subscriptions.clone(),
            from_offsets_excluding: request.from.min_offsets().into(),
            to_offsets_including: present.clone(),
        };
        let bounded = self
            .store
            .bounded_forward(&selection_bounded)
            .await
            .map_err(Error::EventStoreError)?;

        let selection_unbounded = EventSelection::after(tag_subscriptions.clone(), present);
        let unbounded = self.store.unbounded_forward_per_stream(&selection_unbounded);

        let feed = mk_feed(request.query);
        let response = bounded
            .chain(unbounded)
            .flat_map({
                let mut latest = initial_latest;
                move |e| {
                    if e.key > latest {
                        latest = e.key;
                        feed(e)
                            .map(|event| SubscribeMonotonicResponse::Event { event, caught_up: true })
                            .left_stream()
                    } else {
                        stream::once(async move { SubscribeMonotonicResponse::TimeTravel { new_start: e.key } })
                            .right_stream()
                    }
                }
            })
            .take_until_condition(|e| future::ready(matches!(e, SubscribeMonotonicResponse::TimeTravel { .. })));
        Ok(response.boxed())
    }
}

fn mk_feed(query: language::Query) -> impl Fn(Event<Payload>) -> BoxStream<'static, EventResponse<Payload>> {
    let query = runtime::query::Query::from(query);
    move |event| {
        let Event {
            key,
            meta: Metadata { timestamp, tags },
            payload,
        } = event;
        stream::iter(
            query
                .feed(Value::from((key, payload)))
                .into_iter()
                .map(move |v| EventResponse {
                    lamport: v.sort_key.lamport,
                    stream: v.sort_key.stream,
                    offset: v.sort_key.offset,
                    timestamp,
                    tags: tags.clone(),
                    payload: v.payload(),
                }),
        )
        .boxed()
    }
}
