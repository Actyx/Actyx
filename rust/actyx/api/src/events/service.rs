use std::{convert::TryFrom, num::NonZeroU64};

use actyx_sdk::{
    language::{self, TagExpr},
    service::{
        EventResponse, OffsetsResponse, Order, PublishEvent, PublishRequest, PublishResponse, PublishResponseKey,
        QueryRequest, QueryResponse, StartFrom, SubscribeMonotonicRequest, SubscribeMonotonicResponse,
        SubscribeRequest, SubscribeResponse,
    },
    AppId, Event, EventKey, Metadata, OffsetMap, OffsetOrMin, Payload,
};
use ax_futures_util::prelude::*;
use futures::{
    future,
    stream::{self, BoxStream, StreamExt},
    Stream,
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

impl EventService {
    pub async fn offsets(&self) -> anyhow::Result<OffsetsResponse> {
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

    pub async fn publish(&self, _app_id: AppId, request: PublishRequest) -> anyhow::Result<PublishResponse> {
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

    pub async fn query(
        &self,
        _app_id: AppId,
        request: QueryRequest,
    ) -> anyhow::Result<BoxStream<'static, QueryResponse>> {
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

    pub async fn subscribe(
        &self,
        _app_id: AppId,
        request: SubscribeRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeResponse>> {
        Ok(subscribe0(&self.store, &request.query.from, request.offsets)
            .await?
            .flat_map(mk_feed(request.query))
            .map(SubscribeResponse::Event)
            .boxed())
    }

    pub async fn subscribe_monotonic(
        &self,
        _app_id: AppId,
        request: SubscribeMonotonicRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let tag_expr = &request.query.from;

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

        let stream = subscribe0(&self.store, &tag_expr, Some(request.from.min_offsets())).await?;
        let feed = mk_feed(request.query);
        let response = stream
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

async fn subscribe0(
    store: &EventStore,
    tag_expr: &TagExpr,
    offsets: Option<OffsetMap>,
) -> anyhow::Result<impl Stream<Item = Event<Payload>>> {
    let present = store.present().await;
    let bounded = store
        .bounded_forward(tag_expr, offsets, present.clone())
        .await
        .map_err(Error::StoreReadError)?;
    let unbounded = store.unbounded_forward_per_stream(tag_expr, Some(present));
    Ok(bounded.chain(unbounded))
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
