use crate::rejections::ApiError;
use actyx_sdk::{
    app_id,
    language::{self, Arr, SimpleExpr, SpreadExpr},
    service::{
        Diagnostic, OffsetMapResponse, OffsetsResponse, Order, PublishEvent, PublishRequest, PublishResponse,
        PublishResponseKey, QueryRequest, QueryResponse, Severity, StartFrom, SubscribeMonotonicRequest,
        SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
    },
    AppId, Event, EventKey, NodeId, OffsetMap, OffsetOrMin, Payload, StreamNr, TagSet, Timestamp,
};
use ax_futures_util::{stream::AxStreamExt, ReceiverExt};
use futures::{
    future::{poll_fn, ready},
    stream::{self, BoxStream, StreamExt},
    task::noop_waker,
    FutureExt,
};
use genawaiter::sync::{Co, Gen};
use runtime::{
    error::{RuntimeError, RuntimeFailure},
    eval::Context,
    features::{Endpoint, Feature, FeatureError, Features},
    query::{Feeder, Query},
    value::Value,
};
use serde::Deserialize;
use std::{
    convert::TryFrom,
    num::NonZeroU64,
    ops::Deref,
    task::{self, Poll},
};
use swarm::{
    event_store_ref::{EventStoreHandler, EventStoreRef},
    BanyanStore,
};
use tokio::sync::mpsc;

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
        app_id: AppId,
        request: QueryRequest,
    ) -> anyhow::Result<BoxStream<'static, QueryResponse>> {
        let query = language::Query::parse(&request.query).map_err(|e| ApiError::BadRequest {
            cause: format!("{:#}", e),
        })?;

        let (query, pragmas) = Query::from(query, app_id);
        let features = Features::from_query(&query);
        let enabled = query.enabled_features(&pragmas);
        features.validate(&enabled, Endpoint::Query)?;
        let mut feeder = query.make_feeder();

        async fn y(co: &Co<QueryResponse>, vs: Vec<anyhow::Result<Value>>) {
            for v in vs {
                co.yield_(match v {
                    Ok(v) => QueryResponse::Event(v.into()),
                    Err(e) => QueryResponse::Diagnostic(to_diagnostic(e)),
                })
                .await;
            }
        }

        let store = {
            let mut store = None;
            if let Some(value) = pragmas.pragma("events") {
                store = Some(store_ephemeral(value).await?);
            }
            store.unwrap_or_else(|| EphemeralStore(self.store.clone(), None))
        };

        let upper_bound = match request.upper_bound {
            Some(offsets) => offsets,
            None => store.offsets().await?.present(),
        };
        let lower_bound = request.lower_bound.unwrap_or_default();

        let request_order = request.order;
        let gen = Gen::new(move |co: Co<QueryResponse>| async move {
            let cx = Context::root(
                Order::StreamAsc,
                store.clone(),
                lower_bound.clone(),
                upper_bound.clone(),
            );
            let mut cx = cx.child();
            let mut stream = match &query.source {
                language::Source::Events { from, order } => {
                    let order = order.or_else(|| feeder.preferred_order()).unwrap_or(request_order);
                    cx.order = order;
                    let tag_expr = match cx.eval_from(from).await {
                        Ok(t) => t.into_owned(),
                        Err(e) => {
                            return co
                                .yield_(QueryResponse::Diagnostic(Diagnostic::error(e.to_string())))
                                .await
                        }
                    };
                    let stream = match order {
                        Order::Asc => {
                            store
                                .bounded_forward(tag_expr, lower_bound, upper_bound.clone(), false)
                                .await
                        }
                        Order::Desc => store.bounded_backward(tag_expr, lower_bound, upper_bound.clone()).await,
                        Order::StreamAsc => {
                            store
                                .bounded_forward(tag_expr, lower_bound, upper_bound.clone(), true)
                                .await
                        }
                    };
                    let stream = match stream {
                        Ok(s) => s,
                        Err(e) => {
                            return co
                                .yield_(QueryResponse::Diagnostic(Diagnostic::error(e.to_string())))
                                .await
                        }
                    };
                    stream
                        .stop_on_error()
                        .map(|ev| match ev {
                            Ok(ev) => Ok(Value::from(ev)),
                            Err(e) => Err(e.into()),
                        })
                        .left_stream()
                }
                language::Source::Array(Arr { items }) => stream::iter(items.iter())
                    .flat_map(|SpreadExpr { expr, spread }| {
                        let cx = &cx;
                        async move {
                            if let (SimpleExpr::SubQuery(e), true) = (expr, *spread) {
                                match Query::eval(e, cx).await {
                                    Ok(arr) => stream::iter(arr.into_iter().map(Ok)).boxed(),
                                    Err(e) => stream::once(ready(Err(e))).boxed(),
                                }
                            } else {
                                match cx.eval(expr).await {
                                    Ok(val) => {
                                        if *spread {
                                            if let Ok(items) = val.as_array() {
                                                stream::iter(items.into_iter().map(Ok)).boxed()
                                            } else {
                                                stream::once(ready(Err(
                                                    RuntimeError::TypeErrorSpread(val.kind()).into()
                                                )))
                                                .boxed()
                                            }
                                        } else {
                                            stream::once(ready(Ok(val))).boxed()
                                        }
                                    }
                                    Err(e) => stream::once(ready(Err(e))).boxed(),
                                }
                            }
                        }
                        .boxed()
                        .flatten_stream()
                    })
                    .right_stream(),
            };

            while let Some(ev) = stream.next().await {
                let ev = match ev {
                    Ok(ev) => ev,
                    Err(e) => {
                        tracing::error!("aborting query due to {:#}", e);
                        y(&co, vec![Err(e)]).await;
                        return;
                    }
                };
                let vs = feeder.feed(Some(ev), &cx).await;
                y(&co, vs).await;
                if feeder.is_done() {
                    break;
                }
            }
            drop(stream);

            let vs = feeder.feed(None, &cx).await;
            y(&co, vs).await;

            co.yield_(QueryResponse::Offsets(OffsetMapResponse { offsets: upper_bound }))
                .await;
        })
        .take_until_condition(|r| {
            ready(matches!(
                r,
                QueryResponse::Diagnostic(Diagnostic {
                    severity: Severity::Error,
                    ..
                })
            ))
        });

        Ok(gen.boxed())
    }

    pub async fn subscribe(
        &self,
        app_id: AppId,
        request: SubscribeRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeResponse>> {
        let query = language::Query::parse(&request.query).map_err(|e| ApiError::BadRequest {
            cause: format!("{:#}", e),
        })?;

        let (query, pragmas) = Query::from(query, app_id);
        let tag_expr = match &query.source {
            language::Source::Events { from, .. } => from.clone(),
            language::Source::Array(_) => {
                return Err(FeatureError::Unsupported {
                    features: Feature::fromArray.to_string(),
                    endpoint: Endpoint::Subscribe.to_string(),
                }
                .into())
            }
        };
        let present = self.store.offsets().await?.present();
        let mut lower_bound = request.lower_bound.unwrap_or_default();

        let features = Features::from_query(&query);
        let enabled = query.enabled_features(&pragmas);
        features.validate(&enabled, Endpoint::Subscribe)?;
        let mut query = query.make_feeder();

        let cx = Context::root(
            Order::StreamAsc,
            self.store.clone(),
            // no sub-queries supported yet, so no OffsetMap needed
            OffsetMap::empty(),
            OffsetMap::empty(),
        );

        let tag_expr = cx.child().eval_from(&tag_expr).await?.into_owned();
        let tags = tag_expr.clone(); // for logging

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

        async fn y(co: &Co<SubscribeResponse>, vs: Vec<anyhow::Result<Value>>) {
            for v in vs {
                co.yield_(match v {
                    Ok(v) if v.is_anti() => SubscribeResponse::AntiEvent(v.into()),
                    Ok(v) => SubscribeResponse::Event(v.into()),
                    Err(e) => SubscribeResponse::Diagnostic(to_diagnostic(e)),
                })
                .await;
            }
        }

        let gen = Gen::new(move |co: Co<SubscribeResponse>| async move {
            let cx = cx.child();
            while let Some(ev) = bounded.next().await {
                let ev = match ev {
                    Ok(ev) => ev,
                    Err(e) => {
                        tracing::error!("aborting subscribe catch-up for tags {} due to {:#}", tags, e);
                        y(&co, vec![Err(e.into())]).await;
                        return;
                    }
                };
                let vs = query.feed(Some(ev.into()), &cx).await;
                y(&co, vs).await;
            }

            let vs = query.feed(None, &cx).await;
            y(&co, vs).await;

            co.yield_(SubscribeResponse::Offsets(OffsetMapResponse { offsets: present }))
                .await;

            'a: while let Some(mut input) = unbounded.next().await {
                loop {
                    let ev = match input {
                        Ok(ev) => ev,
                        Err(e) => {
                            tracing::error!("aborting subscribe for tags {} due to {:#}", tags, e);
                            y(&co, vec![Err(e.into())]).await;
                            return;
                        }
                    };
                    let vs = query.feed(Some(ev.into()), &cx).await;
                    y(&co, vs).await;
                    if query.is_done() {
                        break 'a;
                    }
                    input = match unbounded.poll_next_unpin(&mut task::Context::from_waker(&noop_waker())) {
                        Poll::Ready(Some(ev)) => ev,
                        Poll::Ready(None) => break 'a,
                        Poll::Pending => break,
                    };
                }
                let vs = query.feed(None, &cx).await;
                y(&co, vs).await;
                if query.is_done() {
                    break;
                }
            }
            if !query.is_done() {
                let vs = query.feed(None, &cx).await;
                y(&co, vs).await;
            }
        })
        .take_until_condition(|r| {
            ready(matches!(
                r,
                SubscribeResponse::Diagnostic(Diagnostic {
                    severity: Severity::Error,
                    ..
                })
            ))
        });

        Ok(gen.boxed())
    }

    pub async fn subscribe_monotonic(
        &self,
        app_id: AppId,
        request: SubscribeMonotonicRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let query = language::Query::parse(&request.query).map_err(|e| ApiError::BadRequest {
            cause: format!("{:#}", e),
        })?;

        let (query, pragmas) = Query::from(query, app_id);
        let tag_expr = match &query.source {
            language::Source::Events { from, .. } => from.clone(),
            language::Source::Array(_) => {
                return Err(FeatureError::Unsupported {
                    features: Feature::fromArray.to_string(),
                    endpoint: Endpoint::Subscribe.to_string(),
                }
                .into())
            }
        };
        let mut lower_bound = match &request.from {
            StartFrom::LowerBound(x) => x.clone(),
        };
        let mut present = self.store.offsets().await?.present();
        present.union_with(&lower_bound);

        let features = Features::from_query(&query);
        let enabled = query.enabled_features(&pragmas);
        features.validate(&enabled, Endpoint::SubscribeMonotonic)?;
        let mut query = query.make_feeder();

        let cx = Context::root(
            Order::Asc,
            self.store.clone(),
            // no sub-queries supported yet, so no OffsetMap needed
            OffsetMap::empty(),
            OffsetMap::empty(),
        );

        let tag_expr = cx.child().eval_from(&tag_expr).await?.into_owned();
        let tags = tag_expr.clone(); // for logging

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
                let vs = query.feed(Some(event.into()), cx).await;
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
                                event: v.into(),
                                caught_up,
                            },
                            Err(e) => SubscribeMonotonicResponse::Diagnostic(to_diagnostic(e)),
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
            let cx = cx.child();
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
        })
        .take_until_condition(|r| {
            ready(matches!(
                r,
                SubscribeMonotonicResponse::Diagnostic(Diagnostic {
                    severity: Severity::Error,
                    ..
                })
            ))
        });

        Ok(gen.boxed())
    }
}

fn to_diagnostic(err: anyhow::Error) -> Diagnostic {
    if let Some(err) = err.downcast_ref::<RuntimeFailure>() {
        Diagnostic {
            severity: Severity::Error,
            message: err.to_string(),
        }
    } else {
        Diagnostic {
            severity: Severity::Warning,
            message: err.to_string(),
        }
    }
}

struct EphemeralStore(EventStoreRef, Option<BanyanStore>);
impl Drop for EphemeralStore {
    fn drop(&mut self) {
        if let Some(store) = &self.1 {
            store.abort_task("handler");
        }
    }
}
impl Deref for EphemeralStore {
    type Target = EventStoreRef;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

async fn store_ephemeral(value: &str) -> anyhow::Result<EphemeralStore> {
    let banyan = BanyanStore::test("query").await.unwrap();
    for line in value.lines() {
        store_line(&banyan, line).await?;
    }
    let event_store = {
        let store2 = banyan.clone();
        let (tx, mut rx) = mpsc::channel(100);
        banyan.spawn_task("handler".to_owned(), async move {
            let mut handler = EventStoreHandler::new(store2);
            let runtime = tokio::runtime::Handle::current();
            while let Some(request) = rx.recv().await {
                handler.handle(request, &runtime);
            }
        });
        EventStoreRef::new(move |e| tx.try_send(e).map_err(swarm::event_store_ref::Error::from))
    };
    Ok(EphemeralStore(event_store, Some(banyan)))
}

async fn store_line(store: &BanyanStore, line: &str) -> anyhow::Result<()> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Line<'a> {
        timestamp: Option<Timestamp>,
        time: Option<&'a str>,
        tags: Option<TagSet>,
        app_id: Option<AppId>,
        payload: Payload,
    }
    let line: Line = serde_json::from_str(line)?;
    let timestamp = line
        .timestamp
        .or_else(|| line.time.and_then(|t| t.parse().ok()))
        .unwrap_or_else(Timestamp::now);
    let app_id = line.app_id.unwrap_or_else(|| app_id!("com.actyx.test"));
    let events = vec![(line.tags.unwrap_or_default(), line.payload)];
    store.append0(0.into(), app_id, timestamp, events).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyx_sdk::{
        app_id,
        service::{EventMeta, EventResponse, SessionId},
        tags, Metadata, Offset, StreamId, TagSet,
    };
    use futures::Stream;
    use itertools::Itertools;
    use lazy_static::lazy_static;
    use maplit::btreemap;
    use regex::Regex;
    use std::{collections::BTreeMap, convert::TryInto, iter::FromIterator, pin::Pin, time::Duration};
    use swarm::{
        event_store_ref::{self, EventStoreHandler},
        BanyanStore,
    };
    use tokio::{
        runtime::{Handle, Runtime},
        sync::mpsc,
        time::timeout,
    };

    #[derive(Debug, PartialEq, Eq)]
    enum SResp {
        Event(String),
        AntiEvent(String),
        Offsets(BTreeMap<u64, u64>),
        Diag(String),
    }

    impl SResp {
        pub fn event(s: &str) -> Self {
            Self::Event(s.into())
        }
        pub fn anti(s: &str) -> Self {
            Self::AntiEvent(s.into())
        }
        pub fn diag(s: &str) -> Self {
            Self::Diag(s.into())
        }
        pub async fn next(mut s: Pin<&mut dyn Stream<Item = SubscribeResponse>>) -> Self {
            Self::from(timeout(Duration::from_millis(500), s.next()).await.unwrap().unwrap())
        }
    }

    impl From<SubscribeResponse> for SResp {
        fn from(sr: SubscribeResponse) -> Self {
            lazy_static! {
                static ref RE: Regex = Regex::new(r"/[^-]+-(\d+)@[^ ]+").unwrap();
            }
            match sr {
                SubscribeResponse::Event(e) => {
                    Self::Event(RE.replace_all(&Value::from(e).to_string(), "-$1").into_owned())
                }
                SubscribeResponse::AntiEvent(e) => {
                    Self::AntiEvent(RE.replace_all(&Value::from(e).to_string(), "-$1").into_owned())
                }
                SubscribeResponse::Offsets(OffsetMapResponse { offsets }) => Self::Offsets(
                    offsets
                        .into_inner()
                        .into_iter()
                        .map(|(k, v)| (u64::from(k.stream_nr()), u64::from(v)))
                        .collect(),
                ),
                SubscribeResponse::Diagnostic(d) => Self::Diag(format!("{:?} {}", d.severity, d.message)),
                x => panic!("unexpected: {:?}", x),
            }
        }
    }

    fn setup(store: &BanyanStore) -> (NodeId, EventService) {
        let event_store = {
            let store2 = store.clone();
            let (tx, mut rx) = mpsc::channel(100);
            store.spawn_task("handler".to_owned(), async move {
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
    async fn publish(service: &EventService, stream: u64, tags: TagSet, data: u32) -> PublishResponseKey {
        let d = service
            .publish(
                app_id!("test"),
                stream.into(),
                PublishRequest {
                    data: vec![evp(tags, data)],
                },
            )
            .await
            .unwrap()
            .data;
        assert_eq!(d.len(), 1);
        d.into_iter().next().unwrap()
    }
    fn evp(tags: TagSet, n: u32) -> PublishEvent {
        PublishEvent {
            tags,
            payload: Payload::from_json_str(&format!("{:?}", n)).unwrap(),
        }
    }
    fn evr(publ: PublishResponseKey, tags: TagSet, n: u32) -> SubscribeResponse {
        SubscribeResponse::Event(EventResponse {
            meta: EventMeta::Event {
                key: EventKey {
                    lamport: publ.lamport,
                    stream: publ.stream,
                    offset: publ.offset,
                },
                meta: Metadata {
                    timestamp: publ.timestamp,
                    tags,
                    app_id: app_id!("test"),
                },
            },
            payload: Payload::from_json_str(&format!("{:?}", n)).unwrap(),
        })
    }
    fn offsets(offsets: OffsetMap) -> SubscribeResponse {
        SubscribeResponse::Offsets(OffsetMapResponse { offsets })
    }
    async fn query(service: &EventService, q: &str) -> Vec<String> {
        service
            .query(
                app_id!("test"),
                QueryRequest {
                    lower_bound: None,
                    upper_bound: None,
                    query: q.to_owned(),
                    order: Order::StreamAsc,
                },
            )
            .await
            .unwrap()
            .map(|x| match x {
                QueryResponse::Event(e) => e.payload.json_string(),
                QueryResponse::Offsets(_) => "offsets".to_owned(),
                QueryResponse::Diagnostic(d) => d.message,
                QueryResponse::FutureCompat => unreachable!(),
            })
            .collect()
            .await
    }
    async fn subscribe(service: &EventService, q: &str) -> Vec<String> {
        service
            .subscribe(
                app_id!("test"),
                SubscribeRequest {
                    lower_bound: None,
                    query: q.to_owned(),
                },
            )
            .await
            .unwrap()
            .take_while(|x| ready(!matches!(x, SubscribeResponse::Offsets(_))))
            .map(|x| match x {
                SubscribeResponse::Event(e) => e.payload.json_string(),
                SubscribeResponse::AntiEvent(e) => format!("-{}", e.payload.json_string()),
                SubscribeResponse::Offsets(_) => "offsets".to_owned(),
                SubscribeResponse::Diagnostic(d) => d.message,
                SubscribeResponse::FutureCompat => unreachable!(),
            })
            .collect()
            .await
    }
    async fn subscribe_monotonic(service: &EventService, q: &str) -> Vec<String> {
        service
            .subscribe_monotonic(
                app_id!("test"),
                SubscribeMonotonicRequest {
                    query: q.to_owned(),
                    session: SessionId::from("wat"),
                    from: StartFrom::LowerBound(OffsetMap::empty()),
                },
            )
            .await
            .unwrap()
            .take_while(|x| ready(!matches!(x, SubscribeMonotonicResponse::Offsets(_))))
            .map(|x| match x {
                SubscribeMonotonicResponse::Event { event, .. } => event.payload.json_string(),
                SubscribeMonotonicResponse::Offsets(_) => "offsets".to_owned(),
                SubscribeMonotonicResponse::TimeTravel { .. } => "timeTravel".to_owned(),
                SubscribeMonotonicResponse::Diagnostic(d) => d.message,
                SubscribeMonotonicResponse::FutureCompat => unreachable!(),
            })
            .collect()
            .await
    }
    async fn values(service: &EventService, q: &str) -> Vec<EventResponse<u64>> {
        service
            .query(
                app_id!("test"),
                QueryRequest {
                    lower_bound: None,
                    upper_bound: None,
                    query: q.to_owned(),
                    order: Order::StreamAsc,
                },
            )
            .await
            .unwrap()
            .flat_map(|x| match x {
                QueryResponse::Event(e) => stream::once(ready(e.extract().unwrap())).left_stream(),
                _ => stream::empty().right_stream(),
            })
            .collect()
            .await
    }

    #[test]
    fn lower_bound() {
        Runtime::new()
            .unwrap()
            .block_on(async {
                timeout(Duration::from_secs(1), async {
                    let store = BanyanStore::test("lower_bound").await.unwrap();
                    let (node_id, service) = setup(&store);

                    let _pub0 = publish(&service, 0, tags!("a"), 0).await;

                    let present = OffsetMap::from_iter(vec![offset(node_id, 0, 0)]);
                    let lower_bound = OffsetMap::from_iter(vec![offset(node_id, 0, 0), offset(node_id, 1, 0)]);

                    let mut stream = service
                        .subscribe(
                            app_id!("test"),
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
                    let _pub1 = publish(&service, 1, tags!("a"), 1).await;
                    // but this is fine
                    let pub2 = publish(&service, 1, tags!("a"), 2).await;
                    assert_eq!(stream.next().await, Some(evr(pub2, tags!("a"), 2)));
                })
                .await
            })
            .unwrap();
    }

    #[test]
    fn limit() {
        Runtime::new()
            .unwrap()
            .block_on(async {
                timeout(Duration::from_secs(1), async {
                    let store = BanyanStore::test("lower_bound").await.unwrap();
                    let (_node_id, service) = setup(&store);

                    publish(&service, 0, tags!("a"), 1).await;
                    publish(&service, 0, tags!("a"), 2).await;
                    publish(&service, 0, tags!("a"), 3).await;

                    assert_eq!(
                        query(
                            &service,
                            "FEATURES(limit zøg aggregate) FROM allEvents LIMIT 2 AGGREGATE FIRST(
                                CASE _ = 2 => _ ENDCASE
                            )"
                        )
                        .await,
                        vec!["no case matched", "2", "offsets"]
                    );
                    assert_eq!(
                        query(
                            &service,
                            "FEATURES(limit zøg aggregate) FROM allEvents LIMIT 2 AGGREGATE FIRST(
                                CASE _ = 3 => _ ENDCASE
                            )"
                        )
                        .await,
                        vec!["no case matched", "no case matched", "no value added", "offsets"]
                    );
                    assert_eq!(
                        query(
                            &service,
                            "FEATURES(limit zøg aggregate) FROM allEvents LIMIT 2 AGGREGATE LAST(
                                CASE _ = 1 => _ ENDCASE
                            )"
                        )
                        .await,
                        vec!["no case matched", "no case matched", "no value added", "offsets"]
                    );
                    assert_eq!(
                        query(
                            &service,
                            "FEATURES(limit zøg aggregate) FROM allEvents ORDER DESC LIMIT 2 AGGREGATE FIRST(
                                CASE _ = 3 => _ ENDCASE
                            )"
                        )
                        .await,
                        vec!["no case matched", "3", "offsets"]
                    );
                    assert_eq!(
                        query(
                            &service,
                            "FEATURES(limit zøg aggregate) FROM allEvents ORDER ASC LIMIT 2 AGGREGATE LAST(
                                CASE _ = 1 => _ ENDCASE
                            )"
                        )
                        .await,
                        vec!["no case matched", "1", "offsets"]
                    );
                })
                .await
            })
            .unwrap();
    }

    #[test]
    fn order() {
        Runtime::new()
            .unwrap()
            .block_on(async {
                timeout(Duration::from_secs(1), async {
                    let store = BanyanStore::test("lower_bound").await.unwrap();
                    let (_node_id, service) = setup(&store);

                    publish(&service, 0, tags!("a"), 1).await;
                    publish(&service, 0, tags!("a"), 2).await;
                    publish(&service, 0, tags!("a"), 3).await;

                    assert_eq!(
                        query(
                            &service,
                            "FEATURES(zøg subQuery) FROM 'a' FILTER _ = 2 SELECT FROM 'a' ORDER DESC"
                        )
                        .await,
                        vec!["[3,2,1]", "offsets"]
                    );
                })
                .await
            })
            .unwrap();
    }

    #[test]
    fn binding() {
        Runtime::new()
            .unwrap()
            .block_on(async {
                timeout(Duration::from_secs(1), async {
                    let store = BanyanStore::test("lower_bound").await.unwrap();
                    let (_node_id, service) = setup(&store);

                    publish(&service, 0, tags!("a", "b"), 1).await;
                    publish(&service, 0, tags!("a", "b"), 2).await;
                    publish(&service, 0, tags!("a", "b"), 3).await;

                    assert_eq!(
                        query(
                            &service,
                            "FEATURES(zøg subQuery binding) \
                            FROM 'a' LET a := _ SELECT \
                                FROM 'b' LET b := _ FILTER b > a LET a := b SELECT b"
                        )
                        .await,
                        vec!["[2,3]", "[3]", "[]", "offsets"]
                    );
                })
                .await
            })
            .unwrap();
    }

    #[test]
    fn interpolation() {
        Runtime::new()
            .unwrap()
            .block_on(async {
                timeout(Duration::from_secs(1), async {
                    let store = BanyanStore::test("lower_bound").await.unwrap();
                    let (_node_id, service) = setup(&store);

                    publish(&service, 0, tags!("a1"), 2).await;
                    publish(&service, 0, tags!("a2"), 3).await;
                    publish(&service, 0, tags!("a3"), 1).await;

                    assert_eq!(
                        query(
                            &service,
                            "FEATURES(zøg subQuery interpolation binding) \
                            FROM 'a1' LET x := _ SELECT \
                                FROM `a{_}` SELECT \
                                    FROM `a{_}` SELECT `x = {x} y = {_}`"
                        )
                        .await,
                        vec!["[[\"x = 2 y = 1\"]]", "offsets"]
                    );

                    assert_eq!(
                        query(
                            &service,
                            r#"PRAGMA features := interpolation
PRAGMA events
{"time":"2011-06-17T18:30+02:00","payload":null}
ENDPRAGMA
                            FROM allEvents SELECT `{(TIME(_))[0]}`
                            "#
                        )
                        .await,
                        vec!["\"2011-06-17T16:30:00.000000Z\"", "offsets"]
                    )
                })
                .await
            })
            .unwrap();
    }

    #[test]
    fn from_array() {
        Runtime::new()
            .unwrap()
            .block_on(async {
                timeout(Duration::from_secs(1), async {
                    let store = BanyanStore::test("lower_bound").await.unwrap();
                    let (_node_id, service) = setup(&store);

                    publish(&service, 0, tags!("a1"), 2).await;
                    publish(&service, 0, tags!("a2"), 3).await;
                    publish(&service, 0, tags!("a3"), 1).await;

                    assert_eq!(
                        query(
                            &service,
                            "FEATURES(zøg subQuery interpolation binding fromArray) \
                            FROM [1, 2, 3] FILTER _ < 3 LET x := 10 + _ SELECT \
                                FROM `a{_}` SELECT { [`{_}={x}`]: _ * 11 }"
                        )
                        .await,
                        vec!["[{\"2=11\":22}]", "[{\"3=12\":33}]", "offsets"]
                    );
                    assert_eq!(
                        query(
                            &service,
                            "FEATURES(zøg subQuery interpolation fromArray) \
                            FROM allEvents FILTER _ < 3 SELECT FROM [`a{_}`, 'b']"
                        )
                        .await,
                        vec!["[\"a2\",\"b\"]", "[\"a1\",\"b\"]", "offsets"]
                    );
                })
                .await
            })
            .unwrap();
    }

    #[test]
    fn spread() {
        Runtime::new()
            .unwrap()
            .block_on(async {
                timeout(Duration::from_secs(1), async {
                    let store = BanyanStore::test("lower_bound").await.unwrap();
                    let (_node_id, service) = setup(&store);

                    assert_eq!(
                        query(
                            &service,
                            "FEATURES(zøg subQuery interpolation fromArray multiEmission spread) \
                            FROM [1, ...FROM ['r', 'd'] SELECT [NULL, `{_}2`] END, [3]] \
                            SELECT \
                                'hello','world',
                                ...
                                CASE IsDefined(_[0]) => _
                                CASE TRUE => ['not an array']
                                ENDCASE,
                                'EPIC' -- no more faith
                            "
                        )
                        .await,
                        vec![
                            "\"hello\"",
                            "\"world\"",
                            "\"not an array\"",
                            "\"EPIC\"",
                            "\"hello\"",
                            "\"world\"",
                            "null",
                            "\"r2\"",
                            "\"EPIC\"",
                            "\"hello\"",
                            "\"world\"",
                            "null",
                            "\"d2\"",
                            "\"EPIC\"",
                            "\"hello\"",
                            "\"world\"",
                            "3",
                            "\"EPIC\"",
                            "offsets",
                        ]
                    );
                })
                .await
            })
            .unwrap();
    }

    #[test]
    fn metadata() {
        Runtime::new()
            .unwrap()
            .block_on(async {
                timeout(Duration::from_secs(1), async {
                    let store = BanyanStore::test("lower_bound").await.unwrap();
                    let (_node_id, service) = setup(&store);

                    fn meta(publ: PublishResponseKey, t: &str) -> EventMeta {
                        EventMeta::Event {
                            key: EventKey {
                                lamport: publ.lamport,
                                stream: publ.stream,
                                offset: publ.offset,
                            },
                            meta: Metadata {
                                timestamp: publ.timestamp,
                                tags: TagSet::from([t.try_into().unwrap()].as_slice()),
                                app_id: app_id!("test"),
                            },
                        }
                    }
                    let pub1 = publish(&service, 0, tags!("a1"), 2).await;
                    let meta1 = meta(pub1, "a1");
                    let pub2 = publish(&service, 0, tags!("a2"), 3).await;
                    let meta2 = meta(pub2, "a2");
                    let pub3 = publish(&service, 0, tags!("a3"), 1).await;
                    let meta3 = meta(pub3, "a3");

                    fn ev<'a>(m: impl IntoIterator<Item = &'a EventMeta>, payload: u64) -> EventResponse<u64> {
                        let mut meta = EventMeta::Synthetic;
                        for m in m {
                            meta += m;
                        }
                        EventResponse { meta, payload }
                    }

                    assert_eq!(
                        values(&service, "FROM allEvents SELECT _ * 2").await,
                        vec![ev([&meta1], 4), ev([&meta2], 6), ev([&meta3], 2)]
                    );
                    assert_eq!(
                        values(&service, "FEATURES(zøg aggregate) FROM allEvents AGGREGATE LAST(_)").await,
                        vec![ev([&meta3], 1)]
                    );
                    assert_eq!(
                        values(&service, "FEATURES(zøg aggregate) FROM allEvents AGGREGATE FIRST(_)").await,
                        vec![ev([&meta1], 2)]
                    );
                    assert_eq!(
                        values(&service, "FEATURES(zøg aggregate) FROM allEvents AGGREGATE MIN(_)").await,
                        vec![ev([&meta3], 1)]
                    );
                    assert_eq!(
                        values(&service, "FEATURES(zøg aggregate) FROM allEvents AGGREGATE MAX(_)").await,
                        vec![ev([&meta2], 3)]
                    );
                    assert_eq!(
                        values(&service, "FEATURES(zøg aggregate) FROM allEvents AGGREGATE SUM(_)").await,
                        vec![ev([&meta1, &meta3], 6)]
                    );
                    assert_eq!(
                        values(&service, "FEATURES(zøg aggregate) FROM allEvents AGGREGATE PRODUCT(_)").await,
                        vec![ev([&meta1, &meta3], 6)]
                    );
                    assert_eq!(
                        values(
                            &service,
                            "FEATURES(zøg aggregate) FROM allEvents AGGREGATE PRODUCT(CASE _ > 1 => _ ENDCASE)"
                        )
                        .await,
                        vec![ev([&meta1, &meta2], 6)]
                    );
                    assert_eq!(
                        values(&service, "FEATURES(zøg fromArray) FROM [42]").await,
                        vec![ev([], 42)]
                    );
                    assert_eq!(
                        values(
                            &service,
                            "FEATURES(zøg fromArray subQuery spread) FROM [42] SELECT ...FROM allEvents"
                        )
                        .await,
                        vec![ev([&meta1], 2), ev([&meta2], 3), ev([&meta3], 1)]
                    );
                    assert_eq!(
                        values(
                            &service,
                            "FEATURES(zøg fromArray subQuery) FROM [FROM 'a1', FROM 'a3'] SELECT _[0]"
                        )
                        .await,
                        vec![ev([&meta1], 2), ev([&meta3], 1)]
                    );
                })
                .await
            })
            .unwrap();
    }

    #[test]
    fn events() {
        Runtime::new()
            .unwrap()
            .block_on(async {
                timeout(Duration::from_secs(1), async {
                    let store = BanyanStore::test("lower_bound").await.unwrap();
                    let (_node_id, service) = setup(&store);

                    assert_eq!(
                        query(
                            &service,
                            r#"PRAGMA features := zøg subQuery interpolation binding fromArray
                            PRAGMA events
                            {"tags":["a1"],"payload":2}
                            {"tags":["a2"],"payload":3}
                            {"tags":["a3"],"payload":1}
ENDPRAGMA
                            FROM [1, 2, 3] FILTER _ < 3 LET x := 10 + _ SELECT
                                FROM `a{_}` SELECT { [`{_}={x}`]: _ * 11 }
                            "#
                        )
                        .await,
                        vec!["[{\"2=11\":22}]", "[{\"3=12\":33}]", "offsets"]
                    );
                })
                .await
            })
            .unwrap();
    }

    #[test]
    fn metadata_access() {
        Runtime::new()
            .unwrap()
            .block_on(async {
                timeout(Duration::from_secs(1), async {
                    let store = BanyanStore::test("lower_bound").await.unwrap();
                    let (_node_id, service) = setup(&store);

                    fn meta(publ: PublishResponseKey, t: &str) -> (EventKey, Metadata) {
                        (
                            EventKey {
                                lamport: publ.lamport,
                                stream: publ.stream,
                                offset: publ.offset,
                            },
                            Metadata {
                                timestamp: publ.timestamp,
                                tags: TagSet::from([t.try_into().unwrap()].as_slice()),
                                app_id: app_id!("test"),
                            },
                        )
                    }

                    let pub1 = publish(&service, 0, tags!("a1", "b"), 2).await;
                    let meta1 = meta(pub1, "a1");
                    let pub2 = publish(&service, 0, tags!("a2"), 3).await;
                    let meta2 = meta(pub2, "a2");
                    let pub3 = publish(&service, 0, tags!("a3"), 1).await;
                    let meta3 = meta(pub3, "a3");

                    let mut node_bytes = String::from("[");
                    node_bytes.push_str(
                        &meta1
                            .0
                            .stream
                            .node_id
                            .as_ref()
                            .iter()
                            .map(ToString::to_string)
                            .join(","),
                    );
                    node_bytes.push(']');

                    assert_eq!(
                        query(
                            &service,
                            r#"PRAGMA features :=
                            FROM 'a1' | 'a2' SELECT [KEY(_), TIME(_), TAGS(_), APP(_)]
                            "#
                        )
                        .await,
                        vec![
                            format!(
                                "[[[0,{},0,0]],[{:?}],[\"a1\",\"b\"],[\"test\"]]",
                                node_bytes,
                                meta1.1.timestamp.as_i64() as f64 / 1e6
                            ),
                            format!(
                                "[[[1,{},0,1]],[{:?}],[\"a2\"],[\"test\"]]",
                                node_bytes,
                                meta2.1.timestamp.as_i64() as f64 / 1e6
                            ),
                            "offsets".to_owned()
                        ]
                    );

                    assert_eq!(
                        query(
                            &service,
                            r#"PRAGMA features := zøg subQuery interpolation fromArray aggregate
                            FROM 'a1' | 'a3' AGGREGATE SUM(_) SELECT [KEY(_), TIME(_), TAGS(_), APP(_)]
                            "#
                        )
                        .await,
                        vec![
                            format!(
                                "[[[0,{},0,0],[2,{},0,2]],[{:?},{:?}],[],[]]",
                                node_bytes,
                                node_bytes,
                                meta1.1.timestamp.as_i64() as f64 / 1e6,
                                meta3.1.timestamp.as_i64() as f64 / 1e6
                            ),
                            "offsets".to_owned()
                        ]
                    );
                })
                .await
            })
            .unwrap();
    }

    #[test]
    fn subscribe_aggregate() {
        let f = async {
            let store = BanyanStore::test("subscribe_aggregate").await.unwrap();
            let (_node_id, service) = setup(&store);

            publish(&service, 0, tags!("b"), 1).await;
            publish(&service, 0, tags!("b"), 2).await;

            let mut q1 = service
                .subscribe(
                    app_id!("test"),
                    SubscribeRequest {
                        lower_bound: None,
                        query: "PRAGMA features := aggregate
                                FROM allEvents AGGREGATE LAST(_)"
                            .to_owned(),
                    },
                )
                .await
                .unwrap();
            let mut q2 = service
                .subscribe(
                    app_id!("ne"),
                    SubscribeRequest {
                        lower_bound: None,
                        query: "PRAGMA features := aggregate
                                FROM 'a' AGGREGATE LAST(_)"
                            .to_owned(),
                    },
                )
                .await
                .unwrap();
            let mut q3 = service
                .subscribe(
                    app_id!("ne"),
                    SubscribeRequest {
                        lower_bound: None,
                        query: "PRAGMA features := aggregate
                                FROM 'a' AGGREGATE LAST(_) AGGREGATE SUM(1)"
                            .to_owned(),
                    },
                )
                .await
                .unwrap();

            assert_eq!(SResp::next(q1.as_mut()).await, SResp::event("1-0 2"));
            assert_eq!(SResp::next(q1.as_mut()).await, SResp::Offsets(btreemap! {0 => 1}));
            assert_eq!(SResp::next(q2.as_mut()).await, SResp::diag("Warning no value added"));
            assert_eq!(SResp::next(q2.as_mut()).await, SResp::Offsets(btreemap! {0 => 1}));
            assert_eq!(SResp::next(q3.as_mut()).await, SResp::diag("Warning no value added"));
            assert_eq!(SResp::next(q3.as_mut()).await, SResp::diag("Warning no value added"));
            assert_eq!(SResp::next(q3.as_mut()).await, SResp::Offsets(btreemap! {0 => 1}));

            publish(&service, 0, tags!("a"), 2).await;
            assert_eq!(SResp::next(q1.as_mut()).await, SResp::anti("1-0 2"));
            assert_eq!(SResp::next(q1.as_mut()).await, SResp::event("2-0 2"));
            assert_eq!(SResp::next(q2.as_mut()).await, SResp::event("2-0 2"));
            assert_eq!(SResp::next(q3.as_mut()).await, SResp::event("synthetic: 1"));

            publish(&service, 0, tags!("a"), 3).await;
            assert_eq!(SResp::next(q1.as_mut()).await, SResp::anti("2-0 2"));
            assert_eq!(SResp::next(q1.as_mut()).await, SResp::event("3-0 3"));
            assert_eq!(SResp::next(q2.as_mut()).await, SResp::anti("2-0 2"));
            assert_eq!(SResp::next(q2.as_mut()).await, SResp::event("3-0 3"));

            publish(&service, 0, tags!("a"), 4).await;
            assert_eq!(SResp::next(q1.as_mut()).await, SResp::anti("3-0 3"));
            assert_eq!(SResp::next(q1.as_mut()).await, SResp::event("4-0 4"));
            assert_eq!(SResp::next(q2.as_mut()).await, SResp::anti("3-0 3"));
            assert_eq!(SResp::next(q2.as_mut()).await, SResp::event("4-0 4"));

            timeout(Duration::from_millis(500), q3.next()).await.unwrap_err();
        };
        Runtime::new()
            .unwrap()
            .block_on(async { timeout(Duration::from_secs(10), f).await })
            .unwrap();
    }

    #[test]
    fn subscribe_aggregate_limit() {
        let f = async {
            let store = BanyanStore::test("subscribe_aggregate").await.unwrap();
            let (_node_id, service) = setup(&store);

            publish(&service, 0, tags!("b"), 1).await;

            let mut q = service
                .subscribe(
                    app_id!("test"),
                    SubscribeRequest {
                        lower_bound: None,
                        query: "PRAGMA features := aggregate spread
                                FROM allEvents
                                AGGREGATE LAST(_)
                                SELECT ...CASE _ < 3 => [_] CASE _ = 3 => [_, _] CASE TRUE => [_, _, _] ENDCASE
                                LIMIT 3"
                            .to_owned(),
                    },
                )
                .await
                .unwrap();

            assert_eq!(SResp::next(q.as_mut()).await, SResp::event("0-0 1"));
            assert_eq!(SResp::next(q.as_mut()).await, SResp::Offsets(btreemap! {0 => 0}));

            publish(&service, 0, tags!("b"), 2).await;
            assert_eq!(SResp::next(q.as_mut()).await, SResp::anti("0-0 1"));
            assert_eq!(SResp::next(q.as_mut()).await, SResp::event("1-0 2"));

            publish(&service, 0, tags!("b"), 3).await;
            assert_eq!(SResp::next(q.as_mut()).await, SResp::anti("1-0 2"));
            assert_eq!(SResp::next(q.as_mut()).await, SResp::event("2-0 3"));
            assert_eq!(SResp::next(q.as_mut()).await, SResp::event("2-0 3"));

            publish(&service, 0, tags!("b"), 4).await;
            assert_eq!(SResp::next(q.as_mut()).await, SResp::anti("2-0 3"));
            assert_eq!(SResp::next(q.as_mut()).await, SResp::anti("2-0 3"));
            assert_eq!(SResp::next(q.as_mut()).await, SResp::event("3-0 4"));
            assert_eq!(SResp::next(q.as_mut()).await, SResp::event("3-0 4"));
            assert_eq!(SResp::next(q.as_mut()).await, SResp::event("3-0 4"));

            assert_eq!(timeout(Duration::from_millis(300), q.next()).await.unwrap(), None);
        };
        Runtime::new()
            .unwrap()
            .block_on(async { timeout(Duration::from_secs(10), f).await })
            .unwrap();
    }

    #[test]
    fn subscribe_aggregate_filter() {
        let f = async {
            let store = BanyanStore::test("subscribe_aggregate").await.unwrap();
            let (_node_id, service) = setup(&store);

            publish(&service, 0, tags!("b"), 1).await;

            let mut q = service
                .subscribe(
                    app_id!("test"),
                    SubscribeRequest {
                        lower_bound: None,
                        query: "PRAGMA features := aggregate spread
                                FROM allEvents
                                AGGREGATE LAST(_)
                                FILTER _ < 3"
                            .to_owned(),
                    },
                )
                .await
                .unwrap();

            assert_eq!(SResp::next(q.as_mut()).await, SResp::event("0-0 1"));
            assert_eq!(SResp::next(q.as_mut()).await, SResp::Offsets(btreemap! {0 => 0}));

            publish(&service, 0, tags!("b"), 2).await;
            assert_eq!(SResp::next(q.as_mut()).await, SResp::anti("0-0 1"));
            assert_eq!(SResp::next(q.as_mut()).await, SResp::event("1-0 2"));

            publish(&service, 0, tags!("b"), 3).await;
            assert_eq!(SResp::next(q.as_mut()).await, SResp::anti("1-0 2"));

            publish(&service, 0, tags!("b"), 1).await;
            assert_eq!(SResp::next(q.as_mut()).await, SResp::event("3-0 1"));
        };
        Runtime::new()
            .unwrap()
            .block_on(async { timeout(Duration::from_secs(10), f).await })
            .unwrap();
    }

    #[test]
    fn subscribe_aggregate_error() {
        let f = async {
            let store = BanyanStore::test("subscribe_aggregate").await.unwrap();
            let (_node_id, service) = setup(&store);

            publish(&service, 0, tags!("b"), 1).await;

            let mut q = service
                .subscribe(
                    app_id!("test"),
                    SubscribeRequest {
                        lower_bound: None,
                        query: "PRAGMA features := aggregate spread
                                FROM allEvents
                                AGGREGATE LAST(_)
                                AGGREGATE MAX(3)"
                            .to_owned(),
                    },
                )
                .await
                .unwrap();

            assert_eq!(SResp::next(q.as_mut()).await, SResp::event("synthetic: 3"));
            assert_eq!(SResp::next(q.as_mut()).await, SResp::Offsets(btreemap! {0 => 0}));

            publish(&service, 0, tags!("b"), 2).await;
            assert_eq!(
                SResp::next(q.as_mut()).await,
                SResp::diag("Error anti-input cannot be processed in MAX()")
            );
            assert_eq!(timeout(Duration::from_millis(300), q.next()).await.unwrap(), None);
        };
        Runtime::new()
            .unwrap()
            .block_on(async { timeout(Duration::from_secs(10), f).await })
            .unwrap();
    }

    #[test]
    fn app_id_me() {
        let f = async {
            let store = BanyanStore::test("subscribe_aggregate").await.unwrap();
            let (_node_id, service) = setup(&store);

            publish(&service, 0, tags!(), 42).await;

            assert_eq!(query(&service, "FROM appId(me)").await, vec!["42", "offsets"]);
            assert_eq!(subscribe(&service, "FROM appId(me)").await, vec!["42"]);
            assert_eq!(subscribe_monotonic(&service, "FROM appId(me)").await, vec!["42"]);
        };
        Runtime::new()
            .unwrap()
            .block_on(async { timeout(Duration::from_secs(10), f).await })
            .unwrap();
    }
}
