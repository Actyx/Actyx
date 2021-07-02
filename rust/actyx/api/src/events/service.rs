use actyx_sdk::{
    language,
    service::{
        EventResponse, OffsetMapResponse, OffsetsResponse, Order, PublishEvent, PublishRequest, PublishResponse,
        PublishResponseKey, QueryRequest, QueryResponse, StartFrom, SubscribeMonotonicRequest,
        SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
    },
    AppId, Event, Metadata, NodeId, OffsetMap, OffsetOrMin, Payload,
};
use ax_futures_util::prelude::AxStreamExt;
use futures::{
    future::{self, ready},
    stream::{self, BoxStream, StreamExt},
};
use runtime::value::Value;
use std::{convert::TryFrom, num::NonZeroU64};
use swarm::event_store_ref::EventStoreRef;
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;

trait ReceiverExt<T> {
    fn stop_on_error(self) -> BoxStream<'static, T>;
}
impl<T: Send + 'static, E: std::fmt::Debug + Send + 'static> ReceiverExt<T> for Receiver<Result<T, E>> {
    fn stop_on_error(self) -> BoxStream<'static, T> {
        ReceiverStream::new(self)
            .take_while(|x| ready(x.is_ok()))
            .map(|x| x.unwrap())
            .boxed()
    }
}

#[derive(Clone)]
pub struct EventService {
    store: EventStoreRef,
    node_id: NodeId,
}

impl EventService {
    pub fn new(store: EventStoreRef, node_id: NodeId) -> EventService {
        EventService { store, node_id }
    }
}

impl EventService {
    pub async fn offsets(&self) -> anyhow::Result<OffsetsResponse> {
        let offsets = self.store.offsets().await?;
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

    pub async fn publish(&self, app_id: AppId, request: PublishRequest) -> anyhow::Result<PublishResponse> {
        let events = request
            .data
            .into_iter()
            .map(|PublishEvent { tags, payload }| (tags, payload))
            .collect();
        let meta = self.store.persist(app_id, events).await?;
        let response = PublishResponse {
            data: meta
                .into_iter()
                .map(|(lamport, offset, stream_nr, timestamp)| PublishResponseKey {
                    lamport,
                    offset,
                    stream: self.node_id.stream(stream_nr),
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
        let tag_expr = request.query.from.clone();
        let upper_bound = match request.upper_bound {
            Some(offsets) => offsets,
            None => self.store.offsets().await?.present(),
        };
        let stream = match request.order {
            Order::Asc => {
                self.store
                    .bounded_forward(
                        tag_expr,
                        request.lower_bound.unwrap_or_default(),
                        upper_bound.clone(),
                        false,
                    )
                    .await?
            }
            Order::Desc => {
                self.store
                    .bounded_backward(tag_expr, request.lower_bound.unwrap_or_default(), upper_bound.clone())
                    .await?
            }
            Order::StreamAsc => {
                self.store
                    .bounded_forward(
                        tag_expr,
                        request.lower_bound.unwrap_or_default(),
                        upper_bound.clone(),
                        true,
                    )
                    .await?
            }
        };
        let response = stream
            .stop_on_error()
            .flat_map(mk_feed(request.query))
            .map(QueryResponse::Event)
            .chain(stream::once(ready(QueryResponse::Offsets(OffsetMapResponse {
                offsets: upper_bound,
            }))))
            .boxed();
        Ok(response)
    }

    pub async fn subscribe(
        &self,
        _app_id: AppId,
        request: SubscribeRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeResponse>> {
        let present = self.store.offsets().await?.present();
        let bounded = self
            .store
            .bounded_forward(
                request.query.from.clone(),
                request.lower_bound.unwrap_or_default(),
                present.clone(),
                false,
            )
            .await?
            .stop_on_error()
            .flat_map(mk_feed(request.query.clone()))
            .map(SubscribeResponse::Event);
        let offsets = stream::once(future::ready(SubscribeResponse::Offsets(OffsetMapResponse {
            offsets: present.clone(),
        })));
        let unbounded = self
            .store
            .unbounded_forward(request.query.from.clone(), present)
            .await?
            .stop_on_error()
            .flat_map(mk_feed(request.query.clone()))
            .map(SubscribeResponse::Event);
        Ok(bounded.chain(offsets).chain(unbounded).boxed())
    }

    pub async fn subscribe_monotonic(
        &self,
        _app_id: AppId,
        request: SubscribeMonotonicRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let present = self.store.offsets().await?.present();
        let from_offsets_excluding = match &request.from {
            StartFrom::LowerBound(x) => x.clone(),
        };

        let bounded = self
            .store
            .bounded_forward(
                request.query.from.clone(),
                from_offsets_excluding,
                present.clone(),
                false,
            )
            .await?
            .stop_on_error()
            .flat_map(mk_feed(request.query.clone()))
            .map(|event| SubscribeMonotonicResponse::Event { event, caught_up: true });

        let offsets = stream::once(ready(SubscribeMonotonicResponse::Offsets(OffsetMapResponse {
            offsets: present.clone(),
        })));

        let feed = mk_feed(request.query.clone());
        let mut latest = match &request.from {
            StartFrom::LowerBound(offsets) => self
                .store
                .bounded_backward(request.query.from.clone(), OffsetMap::default(), offsets.clone())
                .await?
                .recv()
                .await
                .transpose()?
                .map(|event| event.key),
        };

        let unbounded = self
            .store
            .unbounded_forward(request.query.from.clone(), present)
            .await?
            .stop_on_error()
            .flat_map({
                move |e| {
                    let key = Some(e.key);
                    if key > latest {
                        latest = key;
                        feed(e)
                            .map(|event| SubscribeMonotonicResponse::Event { event, caught_up: true })
                            .left_stream()
                    } else {
                        stream::once(async move { SubscribeMonotonicResponse::TimeTravel { new_start: e.key } })
                            .right_stream()
                    }
                }
            })
            .take_until_condition(|e| ready(matches!(e, SubscribeMonotonicResponse::TimeTravel { .. })));

        Ok(bounded.chain(offsets).chain(unbounded).boxed())
    }
}

fn mk_feed(query: language::Query) -> impl Fn(Event<Payload>) -> BoxStream<'static, EventResponse<Payload>> {
    let query = runtime::query::Query::from(query);
    move |event| {
        let Event {
            key,
            meta: Metadata {
                timestamp,
                tags,
                app_id,
            },
            payload,
        } = event;
        stream::iter(
            query
                .feed(Value::from((key, payload)))
                .into_iter()
                .map(move |v| EventResponse {
                    lamport: v.key().lamport,
                    stream: v.key().stream,
                    offset: v.key().offset,
                    app_id: app_id.clone(),
                    timestamp,
                    tags: tags.clone(),
                    payload: v.payload(),
                }),
        )
        .boxed()
    }
}
