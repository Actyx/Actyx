use crate::rejections::ApiError;
use actyx_sdk::{
    app_id,
    language::{self, SortKey},
    service::{
        Diagnostic, EventResponse, OffsetMapResponse, OffsetsResponse, Order, PublishEvent, PublishRequest,
        PublishResponse, PublishResponseKey, QueryRequest, QueryResponse, StartFrom, SubscribeMonotonicRequest,
        SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
    },
    AppId, Event, EventKey, NodeId, OffsetMap, OffsetOrMin, Payload, StreamNr,
};
use ax_futures_util::ReceiverExt;
use futures::{
    future::poll_fn,
    stream::{BoxStream, StreamExt},
    FutureExt,
};
use genawaiter::sync::{Co, Gen};
use runtime::{
    eval::Context,
    features::{Endpoint, Features},
    query::{Feeder, Query},
    value::Value,
};
use std::{convert::TryFrom, num::NonZeroU64, task::Poll};
use swarm::event_store_ref::EventStoreRef;

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
        let query = request
            .query
            .parse::<language::Query>()
            .map_err(|e| ApiError::BadRequest {
                cause: format!("{:#}", e),
            })?;
        let tag_expr = query.from.clone();
        let tags = tag_expr.clone(); // for logging
        let upper_bound = match request.upper_bound {
            Some(offsets) => offsets,
            None => self.store.offsets().await?.present(),
        };
        let lower_bound = request.lower_bound.unwrap_or_default();

        let query = Query::from(query);
        let features = Features::from_query(&query);
        features.validate(&query.features, Endpoint::Query)?;
        let mut query = query.make_feeder();
        let order = query.preferred_order().unwrap_or(request.order);

        let cx = Context::owned(
            SortKey::default(),
            order,
            self.store.clone(),
            lower_bound.clone(),
            upper_bound.clone(),
        );

        let mut stream = match order {
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

        async fn y(co: &Co<QueryResponse>, vs: Vec<anyhow::Result<Value>>, event: Option<&Event<Payload>>) {
            for v in vs {
                co.yield_(match v {
                    Ok(v) => QueryResponse::Event(to_event(v, event)),
                    Err(e) => QueryResponse::Diagnostic(Diagnostic::warn(e.to_string())),
                })
                .await;
            }
        }

        let gen = Gen::new(move |co: Co<QueryResponse>| async move {
            while let Some(ev) = stream.next().await {
                let ev = match ev {
                    Ok(ev) => ev,
                    Err(e) => {
                        tracing::error!("aborting query for tags {} due to {:#}", tags, e);
                        y(&co, vec![Err(e.into())], None).await;
                        return;
                    }
                };
                let vs = query.feed(Some(to_value(&ev)), &cx).await;
                y(&co, vs, Some(&ev)).await;
                if query.is_done() {
                    break;
                }
            }
            drop(stream);

            let vs = query.feed(None, &cx).await;
            y(&co, vs, None).await;

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
        let query = request
            .query
            .parse::<language::Query>()
            .map_err(|e| ApiError::BadRequest {
                cause: format!("{:#}", e),
            })?;
        let present = self.store.offsets().await?.present();
        let mut lower_bound = request.lower_bound.unwrap_or_default();

        let tag_expr = query.from.clone();
        let tags = tag_expr.clone(); // for logging
        let query = Query::from(query);
        let features = Features::from_query(&query);
        features.validate(&query.features, Endpoint::Subscribe)?;
        let mut query = query.make_feeder();

        let cx = Context::owned(
            SortKey::default(),
            Order::StreamAsc,
            self.store.clone(),
            // no sub-queries supported yet, so no OffsetMap needed
            OffsetMap::empty(),
            OffsetMap::empty(),
        );

        let mut bounded = self
            .store
            .bounded_forward(tag_expr.clone(), lower_bound.clone(), present.clone(), false)
            .await?
            .stop_on_error();
        lower_bound.union_with(&present);
        let mut unbounded = self
            .store
            .unbounded_forward(tag_expr, lower_bound)
            .await?
            .stop_on_error();

        async fn y(co: &Co<SubscribeResponse>, vs: Vec<anyhow::Result<Value>>, event: Option<&Event<Payload>>) {
            for v in vs {
                co.yield_(match v {
                    Ok(v) => SubscribeResponse::Event(to_event(v, event)),
                    Err(e) => SubscribeResponse::Diagnostic(Diagnostic::warn(e.to_string())),
                })
                .await;
            }
        }

        let gen = Gen::new(move |co: Co<SubscribeResponse>| async move {
            while let Some(ev) = bounded.next().await {
                let ev = match ev {
                    Ok(ev) => ev,
                    Err(e) => {
                        tracing::error!("aborting subscribe catch-up for tags {} due to {:#}", tags, e);
                        y(&co, vec![Err(e.into())], None).await;
                        return;
                    }
                };
                let vs = query.feed(Some(to_value(&ev)), &cx).await;
                y(&co, vs, Some(&ev)).await;
            }

            co.yield_(SubscribeResponse::Offsets(OffsetMapResponse { offsets: present }))
                .await;

            while let Some(ev) = unbounded.next().await {
                let ev = match ev {
                    Ok(ev) => ev,
                    Err(e) => {
                        tracing::error!("aborting subscribe for tags {} due to {:#}", tags, e);
                        y(&co, vec![Err(e.into())], None).await;
                        return;
                    }
                };
                let vs = query.feed(Some(to_value(&ev)), &cx).await;
                y(&co, vs, Some(&ev)).await;
            }
        });

        Ok(gen.boxed())
    }

    pub async fn subscribe_monotonic(
        &self,
        _app_id: AppId,
        request: SubscribeMonotonicRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let query = request
            .query
            .parse::<language::Query>()
            .map_err(|e| ApiError::BadRequest {
                cause: format!("{:#}", e),
            })?;
        let mut lower_bound = match &request.from {
            StartFrom::LowerBound(x) => x.clone(),
        };
        let mut present = self.store.offsets().await?.present();
        present.union_with(&lower_bound);

        let tag_expr = query.from.clone();
        let tags = tag_expr.clone(); // for logging
        let query = Query::from(query);
        let features = Features::from_query(&query);
        features.validate(&query.features, Endpoint::SubscribeMonotonic)?;
        let mut query = query.make_feeder();

        let cx = Context::owned(
            SortKey::default(),
            Order::Asc,
            self.store.clone(),
            // no sub-queries supported yet, so no OffsetMap needed
            OffsetMap::empty(),
            OffsetMap::empty(),
        );

        let mut bounded = self
            .store
            .bounded_forward(tag_expr.clone(), lower_bound.clone(), present.clone(), false)
            .await?
            .stop_on_error();
        lower_bound.union_with(&present);
        let mut unbounded = self
            .store
            .unbounded_forward(tag_expr.clone(), lower_bound)
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
            query: &mut Feeder,
            cx: &Context<'_>,
        ) -> bool {
            let key = event.key;
            if key > *latest {
                *latest = key;
                let vs = query.feed(Some(to_value(&event)), cx).await;
                if !vs.is_empty() {
                    let last = {
                        let mut l = None;
                        for idx in (0..vs.len()).rev() {
                            if vs[idx].is_ok() {
                                l = Some(idx);
                                break;
                            }
                        }
                        l
                    };
                    for (idx, v) in vs.into_iter().enumerate() {
                        let caught_up = Some(idx) == last && caught_up;
                        co.yield_(match v {
                            Ok(v) => SubscribeMonotonicResponse::Event {
                                event: to_event(v, Some(&event)),
                                caught_up,
                            },
                            Err(e) => SubscribeMonotonicResponse::Diagnostic(Diagnostic::warn(e.to_string())),
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
                let ev = match ev {
                    Ok(ev) => ev,
                    Err(e) => {
                        tracing::error!("aborting subscribe_monotonic catch-up for tags {} due to {:#}", tags, e);
                        co.yield_(SubscribeMonotonicResponse::Diagnostic(Diagnostic::error(e.to_string())))
                            .await;
                        return;
                    }
                };
                if send_and_timetravel(&co, ev, &mut latest, false, &mut query, &cx).await {
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
                let ev = match ev {
                    Ok(ev) => ev,
                    Err(e) => {
                        tracing::error!("aborting subscribe_monotonic for tags {} due to {:#}", tags, e);
                        co.yield_(SubscribeMonotonicResponse::Diagnostic(Diagnostic::error(e.to_string())))
                            .await;
                        return;
                    }
                };
                if send_and_timetravel(&co, ev, &mut latest, next.is_pending(), &mut query, &cx).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use actyx_sdk::{tags, Offset, StreamId};
    use std::{iter::FromIterator, time::Duration};
    use swarm::{
        event_store_ref::{self, EventStoreHandler},
        BanyanStore,
    };
    use tokio::{
        runtime::{Handle, Runtime},
        sync::mpsc,
        time::timeout,
    };

    fn setup(store: &BanyanStore) -> (NodeId, EventService) {
        let event_store = {
            let store2 = store.clone();
            let (tx, mut rx) = mpsc::channel(100);
            store.spawn_task("handler", async move {
                let mut handler = EventStoreHandler::new(store2);
                let runtime = Handle::current();
                while let Some(request) = rx.recv().await {
                    handler.handle(request, &runtime);
                }
            });
            EventStoreRef::new(move |e| tx.try_send(e).map_err(event_store_ref::Error::from))
        };
        let node_id = store.node_id();
        (node_id, EventService::new(event_store, node_id))
    }
    fn offset(node_id: NodeId, stream: u64, offset: u32) -> (StreamId, Offset) {
        (node_id.stream(stream.into()), offset.into())
    }
    async fn publish(service: &EventService, stream: u64, data: u32) -> PublishResponseKey {
        let d = service
            .publish(app_id!("me"), stream.into(), PublishRequest { data: vec![evp(data)] })
            .await
            .unwrap()
            .data;
        assert_eq!(d.len(), 1);
        d.into_iter().next().unwrap()
    }
    fn evp(n: u32) -> PublishEvent {
        PublishEvent {
            tags: tags!("a", "b", "c"),
            payload: Payload::from_json_str(&*format!("{:?}", n)).unwrap(),
        }
    }
    fn evr(publ: PublishResponseKey, n: u32) -> SubscribeResponse {
        SubscribeResponse::Event(EventResponse {
            lamport: publ.lamport,
            stream: publ.stream,
            offset: publ.offset,
            timestamp: publ.timestamp,
            tags: tags!("a", "b", "c"),
            app_id: app_id!("me"),
            payload: Payload::from_json_str(&*format!("{:?}", n)).unwrap(),
        })
    }
    fn offsets(offsets: OffsetMap) -> SubscribeResponse {
        SubscribeResponse::Offsets(OffsetMapResponse { offsets })
    }

    #[test]
    fn lower_bound() {
        Runtime::new()
            .unwrap()
            .block_on(async {
                timeout(Duration::from_secs(1), async {
                    let store = BanyanStore::test("lower_bound").await.unwrap();
                    let (node_id, service) = setup(&store);

                    let _pub0 = publish(&service, 0, 0).await;

                    let present = OffsetMap::from_iter(vec![offset(node_id, 0, 0)]);
                    let lower_bound = OffsetMap::from_iter(vec![offset(node_id, 0, 0), offset(node_id, 1, 0)]);

                    let mut stream = service
                        .subscribe(
                            app_id!("me"),
                            SubscribeRequest {
                                lower_bound: Some(lower_bound.clone()),
                                query: "FROM allEvents".to_owned(),
                            },
                        )
                        .await
                        .unwrap();

                    assert_eq!(stream.next().await, Some(offsets(present)));

                    // this event shall not be delivered, even though it is “newer than present”
                    // because lower_bound contains it
                    let _pub1 = publish(&service, 1, 1).await;
                    // but this is fine
                    let pub2 = publish(&service, 1, 2).await;
                    assert_eq!(stream.next().await, Some(evr(pub2, 2)));
                })
                .await
            })
            .unwrap();
    }
}
