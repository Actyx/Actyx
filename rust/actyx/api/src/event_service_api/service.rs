use std::{convert::TryFrom, num::NonZeroU64};

use actyxos_sdk::{
    language,
    service::{
        self, EventResponse, NodeIdResponse, OffsetsResponse, Order, PublishEvent, PublishRequest, PublishResponse,
        PublishResponseKey, QueryRequest, QueryResponse, StartFrom, SubscribeMonotonicRequest,
        SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
    },
    Event, EventKey, Metadata, OffsetOrMin, Payload,
};
use async_trait::async_trait;
use ax_futures_util::prelude::*;
use futures::{
    future,
    stream::{self, BoxStream, StreamExt},
};
use runtime::value::Value;
use swarm::event_store::{self, EventStore};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Store error while writing: {0}")]
    StoreWriteError(#[from] anyhow::Error),
    #[error("Store error while reading: {0}")]
    StoreReadError(#[from] event_store::Error),
}

#[derive(Clone)]
pub struct EventService {
    store: EventStore,
}

impl EventService {
    pub fn new(store: EventStore) -> EventService {
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
        let offsets = self.store.offsets().next().await.expect("offset stream stopped");
        let present = offsets.present();
        let to_replicate = offsets
            .replication_target()
            .stream_iter()
            .filter_map(|(stream, target)| {
                let actual = present.offset(stream);
                let diff = OffsetOrMin::from(target) - actual;
                u64::try_from(diff).ok().and_then(NonZeroU64::new).map(|o| (stream, o))
            })
            .collect();
        Ok(OffsetsResponse { present, to_replicate })
    }

    async fn publish(&self, request: PublishRequest) -> anyhow::Result<PublishResponse> {
        let events = request
            .data
            .into_iter()
            .map(|PublishEvent { tags, payload }| (tags, payload))
            .collect();
        let meta = self.store.persist(events).await.map_err(Error::StoreWriteError)?;
        let response = PublishResponse {
            data: meta
                .into_iter()
                .map(|(lamport, offset, stream_nr, timestamp)| PublishResponseKey {
                    lamport,
                    offset,
                    stream: self.store.node_id().stream(stream_nr),
                    timestamp,
                })
                .collect(),
        };
        Ok(response)
    }

    async fn query(&self, request: QueryRequest) -> anyhow::Result<BoxStream<'static, QueryResponse>> {
        let tag_expr = &request.query.from;
        let stream = match request.order {
            Order::Asc => self
                .store
                .bounded_forward(tag_expr, request.lower_bound, request.upper_bound)
                .await
                .map(|s| s.boxed()),
            Order::Desc => self
                .store
                .bounded_backward(tag_expr, request.lower_bound, request.upper_bound)
                .await
                .map(|s| s.boxed()),
            Order::StreamAsc => self
                .store
                .bounded_forward_per_stream(tag_expr, request.lower_bound, request.upper_bound)
                .await
                .map(|s| s.boxed()),
        };
        let response = stream
            .map_err(Error::StoreReadError)?
            .flat_map(mk_feed(request.query))
            .map(QueryResponse::Event)
            .boxed();
        Ok(response)
    }

    async fn subscribe(&self, request: SubscribeRequest) -> anyhow::Result<BoxStream<'static, SubscribeResponse>> {
        let tag_expr = &request.query.from;
        let present = self.store.present().await;

        let bounded = self
            .store
            .bounded_forward(tag_expr, request.offsets, present.clone())
            .await
            .map_err(Error::StoreReadError)?;

        let unbounded = self.store.unbounded_forward_per_stream(tag_expr, Some(present));

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
        let tag_expr = &request.query.from;
        let present = self.store.present().await;

        let initial_latest = if let StartFrom::Offsets(offsets) = &request.from {
            self.store
                .bounded_backward(tag_expr, None, offsets.clone())
                .await
                .map_err(Error::StoreReadError)?
                .next()
                .await
                .map(|event| event.key)
                .unwrap_or_default()
        } else {
            EventKey::default()
        };

        let bounded = self
            .store
            .bounded_forward(tag_expr, Some(request.from.min_offsets()), present.clone())
            .await
            .map_err(Error::StoreReadError)?;

        let unbounded = self.store.unbounded_forward_per_stream(tag_expr, Some(present));

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
