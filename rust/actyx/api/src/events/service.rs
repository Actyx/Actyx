use actyx_sdk::{
    app_id,
    service::{
        Diagnostic, EventResponse, OffsetMapResponse, OffsetsResponse, Order, PublishEvent, PublishRequest,
        PublishResponse, PublishResponseKey, QueryRequest, QueryResponse, StartFrom, SubscribeMonotonicRequest,
        SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
    },
    AppId, Event, EventKey, NodeId, OffsetMap, OffsetOrMin, Payload, StreamNr,
};
use futures::{
    future::{poll_fn, ready},
    stream::{BoxStream, StreamExt},
    FutureExt,
};
use genawaiter::sync::{Co, Gen};
use runtime::{
    features::{Endpoint, Features},
    query::Query,
    value::Value,
};
use std::{convert::TryFrom, num::NonZeroU64, task::Poll};
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

    pub async fn publish(
        &self,
        app_id: AppId,
        stream_nr: StreamNr,
        request: PublishRequest,
    ) -> anyhow::Result<PublishResponse> {
        let events = request
            .data
            .into_iter()
            .map(|PublishEvent { tags, payload }| (tags, payload))
            .collect();
        let meta = self.store.persist(app_id, stream_nr, events).await?;
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

        let gen = Gen::new(move |co: Co<QueryResponse>| async move {
            while let Some(ev) = stream.next().await {
                let vs = query.feed(Some(to_value(&ev)));
                for v in vs {
                    co.yield_(match v {
                        Ok(v) => QueryResponse::Event(to_event(v, Some(&ev))),
                        Err(e) => QueryResponse::Diagnostic(Diagnostic::warn(e)),
                    })
                    .await;
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
        _app_id: AppId,
        request: SubscribeRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeResponse>> {
        let present = self.store.offsets().await?.present();
        let lower_bound = request.lower_bound.unwrap_or_default();

        let mut query = Query::from(request.query);
        let tag_expr = query.from.clone();
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
        let gen = Gen::new(move |co: Co<SubscribeResponse>| async move {
            while let Some(ev) = bounded.next().await {
                let vs = query.feed(Some(to_value(&ev)));
                for v in vs {
                    co.yield_(match v {
                        Ok(v) => SubscribeResponse::Event(to_event(v, Some(&ev))),
                        Err(e) => SubscribeResponse::Diagnostic(Diagnostic::warn(e)),
                    })
                    .await;
                }
            }

            co.yield_(SubscribeResponse::Offsets(OffsetMapResponse { offsets: present }))
                .await;

            while let Some(ev) = unbounded.next().await {
                let vs = query.feed(Some(to_value(&ev)));
                for v in vs {
                    co.yield_(match v {
                        Ok(v) => SubscribeResponse::Event(to_event(v, Some(&ev))),
                        Err(e) => SubscribeResponse::Diagnostic(Diagnostic::warn(e)),
                    })
                    .await;
                }
            }
        });

        Ok(gen.boxed())
    }

    pub async fn subscribe_monotonic(
        &self,
        _app_id: AppId,
        request: SubscribeMonotonicRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let lower_bound = match &request.from {
            StartFrom::LowerBound(x) => x.clone(),
        };
        let mut present = self.store.offsets().await?.present();
        present.union_with(&lower_bound);

        let mut query = Query::from(request.query);
        let tag_expr = query.from.clone();
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
                .map(|event| event.key)
                .unwrap_or(EventKey {
                    lamport: 0.into(),
                    stream: Default::default(),
                    offset: 0.into(),
                }),
        };

        async fn send_and_timetravel(
            co: &Co<SubscribeMonotonicResponse>,
            event: Event<Payload>,
            latest: &mut EventKey,
            caught_up: bool,
            query: &mut Query,
        ) -> bool {
            let key = event.key;
            if key > *latest {
                *latest = key;
                let vs = query.feed(Some(to_value(&event)));
                if !vs.is_empty() {
                    let last = vs.len() - 1;
                    for (idx, v) in vs.into_iter().enumerate() {
                        co.yield_(match v {
                            Ok(v) => SubscribeMonotonicResponse::Event {
                                event: to_event(v, Some(&event)),
                                caught_up: idx == last && caught_up,
                            },
                            Err(e) => SubscribeMonotonicResponse::Diagnostic(Diagnostic::warn(e)),
                        })
                        .await;
                    }
                }
                false
            } else {
                co.yield_(SubscribeMonotonicResponse::TimeTravel { new_start: event.key })
                    .await;
                true
            }
        }

        let gen = Gen::new(move |co: Co<SubscribeMonotonicResponse>| async move {
            while let Some(ev) = bounded.next().await {
                if send_and_timetravel(&co, ev, &mut latest, false, &mut query).await {
                    break;
                }
            }

            co.yield_(SubscribeMonotonicResponse::Offsets(OffsetMapResponse {
                offsets: present,
            }))
            .await;

            let mut event = unbounded.next().await;
            while let Some(ev) = event {
                let next = poll_fn(|cx| Poll::Ready(unbounded.next().poll_unpin(cx))).await;
                if send_and_timetravel(&co, ev, &mut latest, !next.is_ready(), &mut query).await {
                    break;
                }
                match next {
                    Poll::Ready(x) => event = x,
                    Poll::Pending => event = unbounded.next().await,
                }
            }
        });

        Ok(gen.boxed())
    }
}

fn to_value(event: &Event<Payload>) -> Value {
    Value::from((event.key, event.payload.clone()))
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
