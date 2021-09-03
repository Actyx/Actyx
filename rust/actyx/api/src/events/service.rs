use actyx_sdk::{AppId, Event, Metadata, NodeId, OffsetMap, OffsetOrMin, Payload, app_id, service::{
        Diagnostic, EventResponse, OffsetMapResponse, OffsetsResponse, Order, PublishEvent, PublishRequest,
        PublishResponse, PublishResponseKey, QueryRequest, QueryResponse, StartFrom, SubscribeMonotonicRequest,
        SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
    }};
use futures::{
    future::ready,
    stream::{BoxStream, StreamExt},
};
use genawaiter::sync::{Co, Gen};
use runtime::{
    features::{Endpoint, Features},
    query::Query,
    value::Value,
};
use std::{convert::TryFrom, num::NonZeroU64};
use swarm::event_store_ref::EventStoreRef;
use tokio::sync::mpsc::Receiver;
use tokio_stream::wrappers::ReceiverStream;

use crate::tag_mapper::TagMapper;

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
    pub fn mapper(&self) -> &TagMapper {
        TagMapper::hardcoded()
    }

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

    pub async fn publish(
        &self,
        app_id: AppId,
        app_version: String,
        request: PublishRequest,
    ) -> anyhow::Result<PublishResponse> {
        let events = request
            .data
            .into_iter()
            .map(|PublishEvent { tags, payload }| {
                self.mapper()
                    .tag_set_write_to_global(&app_id, &app_version, &tags)
                    .map(|tags| (tags, payload))
            })
            .collect::<anyhow::Result<_>>()?;
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
        app_id: AppId,
        app_version: String,
        request: QueryRequest,
    ) -> anyhow::Result<BoxStream<'static, QueryResponse>> {
        let tag_expr = self
            .mapper()
            .tag_expr_to_global(&app_id, &app_version, &request.query.from)?;
        let upper_bound = match request.upper_bound {
            Some(offsets) => offsets,
            None => self.store.offsets().await?.present(),
        };
        let lower_bound = request.lower_bound.unwrap_or_default();

        let mut query = Query::from(request.query);
        let features = Features::from_query(&query);
        features.validate(&query.features, Endpoint::Query)?;

        let mut stream = match request.order {
            Order::Asc => {
                self.store
                    .bounded_forward(tag_expr, lower_bound, upper_bound.clone(), false)
                    .await?
            }
            Order::Desc => {
                self.store
                    .bounded_backward(tag_expr, lower_bound, upper_bound.clone())
                    .await?
            }
            Order::StreamAsc => {
                self.store
                    .bounded_forward(tag_expr, lower_bound, upper_bound.clone(), true)
                    .await?
            }
        }
        .stop_on_error();

        let mapper = self.mapper().clone();
        let gen = Gen::new(move |co: Co<QueryResponse>| async move {
            while let Some(ev) = stream.next().await {
                if let Some(ev) = event_to_local(ev, &app_id, &app_version, &mapper) {
                    let vs = query.feed(Some(to_value(&ev)));
                    for v in vs {
                        co.yield_(match v {
                            Ok(v) => QueryResponse::Event(to_event(v, Some(&ev))),
                            Err(e) => QueryResponse::Diagnostic(Diagnostic::warn(e)),
                        })
                        .await;
                    }
                }
            }

            let vs = query.feed(None);
            for v in vs {
                co.yield_(match v {
                    Ok(v) => QueryResponse::Event(to_event(v, None)),
                    Err(e) => QueryResponse::Diagnostic(Diagnostic::warn(e)),
                })
                .await;
            }

            co.yield_(QueryResponse::Offsets(OffsetMapResponse { offsets: upper_bound }))
                .await;
        });

        Ok(gen.boxed())
    }

    pub async fn subscribe(
        &self,
        app_id: AppId,
        app_version: String,
        request: SubscribeRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeResponse>> {
        let present = self.store.offsets().await?.present();
        let lower_bound = request.lower_bound.unwrap_or_default();

        let mut query = Query::from(request.query);
        let tag_expr = self.mapper().tag_expr_to_global(&app_id, &app_version, &query.from)?;
        let features = Features::from_query(&query);
        features.validate(&query.features, Endpoint::Subscribe)?;

        let mut bounded = self
            .store
            .bounded_forward(tag_expr.clone(), lower_bound, present.clone(), false)
            .await?
            .stop_on_error();
        let mut unbounded = self
            .store
            .unbounded_forward(tag_expr, present.clone())
            .await?
            .stop_on_error();
        let mapper = self.mapper().clone();
        let gen = Gen::new(move |co: Co<SubscribeResponse>| async move {
            while let Some(ev) = bounded.next().await {
                if let Some(ev) = event_to_local(ev, &app_id, &app_version, &mapper) {
                    let vs = query.feed(Some(to_value(&ev)));
                    for v in vs {
                        co.yield_(match v {
                            Ok(v) => SubscribeResponse::Event(to_event(v, Some(&ev))),
                            Err(e) => SubscribeResponse::Diagnostic(Diagnostic::warn(e)),
                        })
                        .await;
                    }
                }
            }

            co.yield_(SubscribeResponse::Offsets(OffsetMapResponse { offsets: present }))
                .await;

            while let Some(ev) = unbounded.next().await {
                if let Some(ev) = event_to_local(ev, &app_id, &app_version, &mapper) {
                    let vs = query.feed(Some(to_value(&ev)));
                    for v in vs {
                        co.yield_(match v {
                            Ok(v) => SubscribeResponse::Event(to_event(v, Some(&ev))),
                            Err(e) => SubscribeResponse::Diagnostic(Diagnostic::warn(e)),
                        })
                        .await;
                    }
                }
            }
        });

        Ok(gen.boxed())
    }

    pub async fn subscribe_monotonic(
        &self,
        app_id: AppId,
        app_version: String,
        request: SubscribeMonotonicRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let present = self.store.offsets().await?.present();
        let lower_bound = match &request.from {
            StartFrom::LowerBound(x) => x.clone(),
        };

        let mut query = Query::from(request.query);
        let tag_expr = self.mapper().tag_expr_to_global(&app_id, &app_version, &query.from)?;
        let features = Features::from_query(&query);
        features.validate(&query.features, Endpoint::SubscribeMonotonic)?;

        let mut bounded = self
            .store
            .bounded_forward(tag_expr.clone(), lower_bound, present.clone(), false)
            .await?
            .stop_on_error();
        let mut unbounded = self
            .store
            .unbounded_forward(tag_expr.clone(), present.clone())
            .await?
            .stop_on_error();
        let mut latest = match &request.from {
            StartFrom::LowerBound(offsets) => self
                .store
                .bounded_backward(tag_expr, OffsetMap::default(), offsets.clone())
                .await?
                .recv()
                .await
                .transpose()?
                .map(|event| event.key),
        };

        let mapper = self.mapper().clone();
        let gen = Gen::new(move |co: Co<SubscribeMonotonicResponse>| async move {
            while let Some(ev) = bounded.next().await {
                if let Some(ev) = event_to_local(ev, &app_id, &app_version, &mapper) {
                    let vs = query.feed(Some(to_value(&ev)));
                    for v in vs {
                        co.yield_(match v {
                            Ok(v) => SubscribeMonotonicResponse::Event {
                                event: to_event(v, Some(&ev)),
                                caught_up: true,
                            },
                            Err(e) => SubscribeMonotonicResponse::Diagnostic(Diagnostic::warn(e)),
                        })
                        .await;
                    }
                }
            }

            co.yield_(SubscribeMonotonicResponse::Offsets(OffsetMapResponse {
                offsets: present,
            }))
            .await;

            while let Some(ev) = unbounded.next().await {
                if let Some(ev) = event_to_local(ev, &app_id, &app_version, &mapper) {
                    let key = Some(ev.key);
                    if key > latest {
                        latest = key;
                        let vs = query.feed(Some(to_value(&ev)));
                        for v in vs {
                            co.yield_(match v {
                                Ok(v) => SubscribeMonotonicResponse::Event {
                                    event: to_event(v, Some(&ev)),
                                    caught_up: true,
                                },
                                Err(e) => SubscribeMonotonicResponse::Diagnostic(Diagnostic::warn(e)),
                            })
                            .await;
                        }
                    } else {
                        co.yield_(SubscribeMonotonicResponse::TimeTravel { new_start: ev.key })
                            .await;
                        return;
                    }
                }
            }
        });

        Ok(gen.boxed())
    }
}

fn event_to_local(ev: Event<Payload>, target_app_id: &AppId, target_app_version: &String, mapper: &TagMapper) -> Option<Event<Payload>> {
    let Event {
        key,
        payload,
        meta:
            Metadata {
                timestamp,
                tags,
                app_id,
            },
    } = ev;
    let tags = mapper.tag_set_to_local(target_app_id, target_app_version, &tags);
    if !tags.is_empty() {
        Some(Event {
            key,
            payload,
            meta: Metadata {
                timestamp,
                tags,
                app_id,
            },
        })
    } else {
        None
    }
}

fn to_value(event: &Event<Payload>) -> Value {
    Value::from((event.key.clone(), event.payload.clone()))
}
fn to_event(value: Value, event: Option<&Event<Payload>>) -> EventResponse<Payload> {
    match event {
        Some(event) => EventResponse {
            lamport: event.key.lamport,
            stream: event.key.stream,
            offset: event.key.offset,
            app_id: event.meta.app_id.clone(),
            timestamp: event.meta.timestamp,
            tags: event.meta.tags.clone(),
            payload: value.payload(),
        },
        None => EventResponse {
            lamport: Default::default(),
            stream: Default::default(),
            offset: Default::default(),
            app_id: app_id!("none"),
            timestamp: Default::default(),
            tags: Default::default(),
            payload: value.payload(),
        },
    }
}
