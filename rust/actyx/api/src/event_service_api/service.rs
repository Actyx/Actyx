use std::convert::TryInto;

use actyxos_sdk::{
    language,
    service::{
        self, EventResponse, NodeIdResponse, Order, PublishEvent, PublishRequest, PublishResponse, PublishResponseKey,
        QueryRequest, QueryResponse, StartFrom, SubscribeMonotonicRequest, SubscribeMonotonicResponse,
        SubscribeRequest, SubscribeResponse,
    },
    Event, EventKey, Metadata, OffsetMap, Payload,
};
use async_trait::async_trait;
use ax_futures_util::prelude::*;
use futures::{
    future,
    stream::{self, BoxStream, StreamExt},
    TryFutureExt,
};
use runtime::value::Value;
use swarm::access::{ConsumerAccessError, EventSelection, EventStoreConsumerAccess};
use swarm::{BanyanStore, EventStore, Present};
use thiserror::Error;
use trees::{OffsetMapOrMax, TagSubscriptions};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Store error: {0}")]
    StoreError(#[from] anyhow::Error),
    #[error("Access error: {0}")]
    ConsumerAccesError(#[from] ConsumerAccessError),
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

    async fn offsets(&self) -> anyhow::Result<OffsetMap> {
        let response = self
            .store
            .stream()
            .next()
            .await
            .and_then(|o| o.try_into().ok())
            .unwrap_or_default();
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
            Order::Asc => self.store.stream_events_forward(selection),
            Order::Desc => self.store.stream_events_backward(selection),
            Order::StreamAsc => self.store.stream_events_source_ordered(selection),
        }
        .await
        .map_err(Error::ConsumerAccesError)?
        .flat_map(mk_feed(request.query))
        .map(QueryResponse::Event);
        Ok(response.boxed())
    }

    async fn subscribe(&self, request: SubscribeRequest) -> anyhow::Result<BoxStream<'static, SubscribeResponse>> {
        let tag_subscriptions: TagSubscriptions = (&request.query).into();
        let from_offsets_excluding: OffsetMapOrMax = request.offsets.unwrap_or_default().into();
        let selection = EventSelection::after(tag_subscriptions, from_offsets_excluding);
        let response = self
            .store
            .stream_events_source_ordered(selection)
            .await
            .map_err(Error::ConsumerAccesError)?
            .flat_map(mk_feed(request.query))
            .map(SubscribeResponse::Event);

        Ok(response.boxed())
    }

    async fn subscribe_monotonic(
        &self,
        request: SubscribeMonotonicRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let tag_subscriptions: TagSubscriptions = (&request.query).into();
        let initial_latest = if let StartFrom::Offsets(offsets) = &request.from {
            let to_offsets_including = OffsetMapOrMax::from(offsets.clone());
            let selection = EventSelection::upto(tag_subscriptions.clone(), to_offsets_including);
            let (youngest_opt, _) = self
                .store
                .stream_events_backward(selection)
                .await
                .map_err(Error::ConsumerAccesError)?
                .into_future()
                .await;
            if let Some(youngest) = youngest_opt {
                youngest.key
            } else {
                EventKey::default()
            }
        } else {
            EventKey::default()
        };

        let from_offsets_excluding = request.from.min_offsets().into();
        let selection = EventSelection::after(tag_subscriptions, from_offsets_excluding);
        let feed = mk_feed(request.query);
        let response = self
            .store
            .stream_events_source_ordered(selection)
            .await
            .map_err(Error::ConsumerAccesError)?
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
