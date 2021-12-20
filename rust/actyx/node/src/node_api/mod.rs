use crate::{
    components::{
        node_api::NodeApiSettings,
        store::{Store, StoreRequest, StoreTx},
        Component, ComponentRequest,
    },
    formats::ExternalEvent,
    settings::{SettingsRequest, SYSTEM_SCOPE},
    ShutdownReason,
};
use actyx_sdk::{
    app_id,
    service::{QueryResponse, SubscribeMonotonicResponse, SubscribeResponse},
    tag, LamportTimestamp, NodeId, Payload,
};
use anyhow::{anyhow, bail, Context};
use api::EventService;
use ax_futures_util::stream::{variable::Variable, MergeUnordered};
use cbor_data::Cbor;
use crossbeam::channel::Sender;
use crypto::PublicKey;
use formats::NodesRequest;
use futures::{
    future::{ready, AbortHandle, Abortable, BoxFuture},
    stream::{self, BoxStream, FuturesUnordered},
    task::{self, Poll},
    Future, FutureExt, Stream, StreamExt,
};
use libipld::{cbor::DagCborCodec, codec::Codec, Cid};
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed},
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    identity,
    multiaddr::Protocol,
    ping::{Ping, PingConfig, PingEvent},
    request_response::{
        ProtocolSupport, RequestResponse, RequestResponseConfig, RequestResponseEvent, RequestResponseMessage,
        ResponseChannel,
    },
    swarm::{
        IntoProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters,
        ProtocolsHandler, Swarm, SwarmBuilder, SwarmEvent,
    },
    Multiaddr, NetworkBehaviour, PeerId,
};
use libp2p_streaming_response::{ChannelId, StreamingResponse, StreamingResponseConfig, StreamingResponseEvent};
use parking_lot::Mutex;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::TryFrom,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::Duration,
};
use swarm::{
    event_store_ref::EventStoreRef, BanyanConfig, BlockWriter, StorageConfig, StorageService, StorageServiceStore,
    StorageServiceStoreWrite, StreamAlias,
};
use tokio::{
    sync::oneshot,
    time::{timeout_at, Instant},
};
use trees::{
    tags::{ScopedTag, ScopedTagSet, TagScope},
    AxKey, AxTreeHeader,
};
use util::SocketAddrHelper;
use util::{
    formats::{
        admin_protocol::{AdminProtocol, AdminRequest, AdminResponse},
        banyan_protocol::{
            decode_dump_frame, decode_dump_header, BanyanProtocol, BanyanProtocolName, BanyanRequest, BanyanResponse,
        },
        events_protocol::{EventsProtocol, EventsRequest, EventsResponse},
        ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt, NodeErrorContext, NodesInspectResponse,
    },
    trace_poll::TracePoll,
};
use zstd::stream::write::Decoder;

pub mod formats;

type PendingRequest = BoxFuture<'static, (ChannelId, ActyxOSResult<AdminResponse>)>;
type PendingStream = BoxStream<'static, (ChannelId, Option<EventsResponse>)>;
type PendingFinalise = BoxFuture<'static, (ResponseChannel<BanyanResponse>, BanyanResponse)>;

struct BanyanWriter {
    txn: swarm::BanyanTransaction<swarm::TT, StorageServiceStore, StorageServiceStoreWrite>,
    own: swarm::StreamBuilder<swarm::TT, Payload>,
    other: swarm::StreamBuilder<swarm::TT, Payload>,
    buf: Decoder<'static, Vec<u8>>,
    node_id: Option<NodeId>,
    lamport: LamportTimestamp,
}

impl BanyanWriter {
    fn new(forest: swarm::BanyanForest<swarm::TT, StorageServiceStore>) -> Self {
        let config = BanyanConfig::default();
        Self {
            txn: forest.transaction(|s| {
                let w = s.write().unwrap();
                (s, w)
            }),
            own: swarm::StreamBuilder::new(config.tree.clone(), config.secret.clone()),
            other: swarm::StreamBuilder::new(config.tree, config.secret),
            buf: Decoder::new(Vec::new()).unwrap(),
            node_id: None,
            lamport: LamportTimestamp::default(),
        }
    }
}

struct State {
    store_dir: PathBuf,
    node_tx: Sender<ExternalEvent>,
    node_id: NodeId,
    auth_info: Arc<Mutex<NodeApiSettings>>,
    store: StoreTx,
    events: EventService,
    /// Pending inflight requests to Node.
    pending_oneshot: FuturesUnordered<PendingRequest>,
    pending_stream: MergeUnordered<PendingStream, stream::Empty<PendingStream>>,
    pending_finalise: FuturesUnordered<PendingFinalise>,
    stream_handles: BTreeMap<ChannelId, AbortHandle>,
    admin_sockets: Variable<BTreeSet<Multiaddr>>,
    banyan_stores: BTreeMap<String, BanyanWriter>,
}

#[derive(NetworkBehaviour)]
#[behaviour(poll_method = "poll", out_event = "()")]
pub struct ApiBehaviour {
    admin: StreamingResponse<AdminProtocol>,
    events: StreamingResponse<EventsProtocol>,
    banyan: RequestResponse<BanyanProtocol>,
    ping: Ping,
    identify: Identify,
    #[behaviour(ignore)]
    state: State,
}
type WrappedBehaviour = Swarm<ApiBehaviour>;

macro_rules! request_oneshot {
    ($channel_id:expr, $slf:expr, $build_request:expr, $result:expr) => {{
        let maybe_add_key = $slf.maybe_add_key($channel_id.peer());
        let (tx, rx) = tokio::sync::oneshot::channel();
        $slf.state.node_tx.send($build_request(tx)).unwrap();
        let fut = async move {
            if let Err(e) = maybe_add_key.await {
                tracing::error!("Error adding initial key {}", e);
            }

            let result = rx
                .await
                .ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "Error waiting for response")
                .and_then(|x| x.map($result));

            ($channel_id, result)
        }
        .boxed();
        $slf.state.pending_oneshot.push(fut);
    }};
}

impl ApiBehaviour {
    fn new(
        node_id: NodeId,
        node_tx: Sender<ExternalEvent>,
        store_dir: PathBuf,
        store: StoreTx,
        auth_info: Arc<Mutex<NodeApiSettings>>,
        local_public_key: libp2p::core::PublicKey,
    ) -> Self {
        let tx = store.clone();
        let events = EventStoreRef::new(move |req| {
            tx.try_send(ComponentRequest::Individual(StoreRequest::EventsV2(req)))
                .map_err(swarm::event_store_ref::Error::from)
        });
        let events = EventService::new(events, node_id);
        let state = State {
            node_tx,
            node_id,
            store,
            store_dir,
            events,
            auth_info,
            pending_oneshot: FuturesUnordered::new(),
            pending_stream: MergeUnordered::without_input(),
            pending_finalise: FuturesUnordered::new(),
            stream_handles: BTreeMap::default(),
            admin_sockets: Variable::default(),
            banyan_stores: BTreeMap::default(),
        };
        let mut request_response_config = RequestResponseConfig::default();
        request_response_config.set_request_timeout(Duration::from_secs(120));
        Self {
            ping: Ping::new(PingConfig::new().with_keep_alive(true)),
            admin: StreamingResponse::new(StreamingResponseConfig::default()),
            banyan: RequestResponse::new(
                BanyanProtocol::default(),
                [(BanyanProtocolName, ProtocolSupport::Inbound)],
                request_response_config,
            ),
            events: StreamingResponse::new(StreamingResponseConfig::default()),
            identify: Identify::new(IdentifyConfig::new("Actyx".to_owned(), local_public_key)),
            state,
        }
    }

    /// Checks whether `peer` is authorized to use this API. If there are no
    /// authorized keys, any connected peer is authorized.
    fn is_authorized(&self, peer: &PeerId) -> bool {
        let g = self.state.auth_info.lock();
        g.authorized_keys.is_empty() || g.authorized_keys.contains(peer)
    }

    fn maybe_add_key(&self, peer: PeerId) -> BoxFuture<'static, ActyxOSResult<()>> {
        let mut auth_info = self.state.auth_info.lock();
        if auth_info.authorized_keys.is_empty() {
            match PublicKey::try_from(peer) {
                Ok(key_id) => {
                    tracing::debug!("Adding {} (peer {}) to authorized users", key_id, peer);
                    // Directly add the peer. This will be overridden as soon as the settings round
                    // tripped.
                    auth_info.authorized_keys.push(peer);
                    drop(auth_info);
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    self.state
                        .node_tx
                        .send(ExternalEvent::SettingsRequest(SettingsRequest::SetSettings {
                            scope: format!("{}/admin/authorizedUsers", SYSTEM_SCOPE).parse().unwrap(),
                            ignore_errors: false,
                            json: serde_json::json!([format!("{}", key_id)]),
                            response: tx,
                        }))
                        .unwrap();
                    async move {
                        rx.await
                            .ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "Error waiting for response")
                            .and_then(|x| {
                                x.map(|_| {
                                    tracing::info!(
                                        "User with public key {} has been added as the first authorized user.",
                                        key_id
                                    );
                                })
                            })
                    }
                    .boxed()
                }
                Err(e) => {
                    async move { Err(ActyxOSError::internal(format!("Error converting to PublicKey: {}", e))) }.boxed()
                }
            }
        } else {
            async move { Ok(()) }.boxed()
        }
    }

    // Assumes peer is authorized
    fn enqueue(&mut self, channel_id: ChannelId, request: AdminRequest) {
        match request {
            AdminRequest::NodesLs => request_oneshot!(
                channel_id,
                self,
                |tx| ExternalEvent::NodesRequest(NodesRequest::Ls(tx)),
                AdminResponse::NodesLsResponse
            ),

            AdminRequest::SettingsGet { no_defaults, scope } => request_oneshot!(
                channel_id,
                self,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::GetSettings {
                    no_defaults,
                    scope,
                    response: tx
                }),
                AdminResponse::SettingsGetResponse
            ),
            AdminRequest::SettingsSchema { scope } => request_oneshot!(
                channel_id,
                self,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::GetSchema { scope, response: tx }),
                AdminResponse::SettingsSchemaResponse
            ),
            AdminRequest::SettingsScopes => request_oneshot!(
                channel_id,
                self,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::GetSchemaScopes { response: tx }),
                AdminResponse::SettingsScopesResponse
            ),
            AdminRequest::SettingsSet {
                ignore_errors,
                json,
                scope,
            } => request_oneshot!(
                channel_id,
                self,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::SetSettings {
                    scope,
                    json,
                    ignore_errors,
                    response: tx
                }),
                AdminResponse::SettingsSetResponse
            ),
            AdminRequest::SettingsUnset { scope } => request_oneshot!(
                channel_id,
                self,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::UnsetSettings { scope, response: tx }),
                |_| AdminResponse::SettingsUnsetResponse
            ),
            AdminRequest::NodesInspect => {
                let (tx, rx) = tokio::sync::oneshot::channel();
                self.state
                    .store
                    .send(ComponentRequest::Individual(StoreRequest::NodesInspect(tx)))
                    .unwrap();
                let maybe_add_key = self.maybe_add_key(channel_id.peer());
                let admin_addrs = self
                    .state
                    .admin_sockets
                    .get_cloned()
                    .iter()
                    .map(|a| a.to_string())
                    .collect();
                let fut = async move {
                    if let Err(e) = maybe_add_key.await {
                        tracing::error!("Error adding initial key {}", e);
                    }
                    let res = rx
                        .await
                        .ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "Error waiting for response")
                        .and_then(|x| {
                            x.ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "Error getting swarm state")
                                .map(|res| {
                                    AdminResponse::NodesInspectResponse(NodesInspectResponse {
                                        peer_id: res.peer_id,
                                        swarm_addrs: res.swarm_addrs,
                                        announce_addrs: res.announce_addrs,
                                        admin_addrs,
                                        connections: res.connections,
                                        known_peers: res.known_peers,
                                    })
                                })
                        });
                    (channel_id, res)
                }
                .boxed();
                self.state.pending_oneshot.push(fut);
            }
            AdminRequest::NodesShutdown => {
                self.state
                    .node_tx
                    .send(ExternalEvent::ShutdownRequested(ShutdownReason::TriggeredByUser))
                    .unwrap();
            }
        }
    }

    fn wrap(
        &self,
        c: ChannelId,
        f: impl Future<Output = impl Stream<Item = (ChannelId, Option<EventsResponse>)> + Send + 'static> + Send + 'static,
    ) -> (BoxStream<'static, (ChannelId, Option<EventsResponse>)>, AbortHandle) {
        let mac = self.maybe_add_key(c.peer());
        let (handle, reg) = AbortHandle::new_pair();
        let c2 = c.clone();
        let s = async move {
            match mac.await {
                Ok(_) => f.await.left_stream(),
                Err(e) => {
                    stream::once(ready((c2, Some(EventsResponse::Error { message: e.to_string() })))).right_stream()
                }
            }
        }
        .flatten_stream()
        .chain(stream::once(ready((c, None))));
        let s = Abortable::new(s, reg);
        (s.boxed(), handle)
    }

    fn enqueue_events_v2(&mut self, channel_id: ChannelId, request: EventsRequest) {
        let channel_id2 = channel_id.clone();
        let events = self.state.events.clone();
        let (s, h) = match request {
            EventsRequest::Offsets => self.wrap(channel_id.clone(), async move {
                match events.offsets().await {
                    Ok(o) => stream::once(ready((channel_id, Some(EventsResponse::Offsets(o))))),
                    Err(e) => stream::once(ready((
                        channel_id,
                        Some(EventsResponse::Error { message: e.to_string() }),
                    ))),
                }
            }),
            EventsRequest::Query(request) => self.wrap(channel_id.clone(), async move {
                match events.query(app_id!("com.actyx.cli"), request).await {
                    Ok(resp) => TracePoll::new(
                        resp.filter_map(move |x| {
                            tracing::trace!("got query response {:?}", x);
                            match x {
                                QueryResponse::Event(ev) => {
                                    let span = tracing::trace_span!("ready event");
                                    let _enter = span.enter();
                                    ready(Some((channel_id.clone(), Some(EventsResponse::Event(ev)))))
                                }
                                QueryResponse::Offsets(o) => ready(Some((
                                    channel_id.clone(),
                                    Some(EventsResponse::OffsetMap { offsets: o.offsets }),
                                ))),
                                QueryResponse::Diagnostic(d) => {
                                    ready(Some((channel_id.clone(), Some(EventsResponse::Diagnostic(d)))))
                                }
                                QueryResponse::FutureCompat => ready(None),
                            }
                        }),
                        "node_api events query",
                    )
                    .left_stream(),
                    Err(e) => stream::once(ready((
                        channel_id,
                        Some(EventsResponse::Error { message: e.to_string() }),
                    )))
                    .right_stream(),
                }
            }),
            EventsRequest::Subscribe(request) => self.wrap(channel_id.clone(), async move {
                match events.subscribe(app_id!("com.actyx.cli"), request).await {
                    Ok(resp) => resp
                        .filter_map(move |x| match x {
                            SubscribeResponse::Event(ev) => {
                                ready(Some((channel_id.clone(), Some(EventsResponse::Event(ev)))))
                            }
                            SubscribeResponse::Offsets(o) => ready(Some((
                                channel_id.clone(),
                                Some(EventsResponse::OffsetMap { offsets: o.offsets }),
                            ))),
                            SubscribeResponse::Diagnostic(d) => {
                                ready(Some((channel_id.clone(), Some(EventsResponse::Diagnostic(d)))))
                            }
                            SubscribeResponse::FutureCompat => ready(None),
                        })
                        .left_stream(),
                    Err(e) => stream::once(ready((
                        channel_id,
                        Some(EventsResponse::Error { message: e.to_string() }),
                    )))
                    .right_stream(),
                }
            }),
            EventsRequest::SubscribeMonotonic(request) => self.wrap(channel_id.clone(), async move {
                match events.subscribe_monotonic(app_id!("com.actyx.cli"), request).await {
                    Ok(resp) => resp
                        .filter_map(move |x| match x {
                            SubscribeMonotonicResponse::Offsets(o) => ready(Some((
                                channel_id.clone(),
                                Some(EventsResponse::OffsetMap { offsets: o.offsets }),
                            ))),
                            SubscribeMonotonicResponse::Event { event, .. } => {
                                ready(Some((channel_id.clone(), Some(EventsResponse::Event(event)))))
                            }
                            SubscribeMonotonicResponse::TimeTravel { .. } => ready(Some((channel_id.clone(), None))),
                            SubscribeMonotonicResponse::Diagnostic(d) => {
                                ready(Some((channel_id.clone(), Some(EventsResponse::Diagnostic(d)))))
                            }
                            SubscribeMonotonicResponse::FutureCompat => ready(None),
                        })
                        .left_stream(),
                    Err(e) => stream::once(ready((
                        channel_id,
                        Some(EventsResponse::Error { message: e.to_string() }),
                    )))
                    .right_stream(),
                }
            }),
            EventsRequest::Publish(request) => self.wrap(channel_id.clone(), async move {
                match events.publish(app_id!("com.actyx.cli"), 0.into(), request).await {
                    Ok(resp) => stream::once(ready((channel_id, Some(EventsResponse::Publish(resp))))),
                    Err(e) => stream::once(ready((
                        channel_id,
                        Some(EventsResponse::Error { message: e.to_string() }),
                    ))),
                }
            }),
        };
        self.state.stream_handles.insert(channel_id2, h);
        self.state.pending_stream.push(s);
    }

    /// The main purpose of this function is to shovel responses from any
    /// pending requests to libp2p.
    fn poll(&mut self, cx: &mut task::Context, _: &mut impl PollParameters) ->
    Poll<NetworkBehaviourAction<<<<Self as
    NetworkBehaviour>::ProtocolsHandler as IntoProtocolsHandler>::Handler as
    ProtocolsHandler>::InEvent, ()>>{
        let mut wake_me_up = false;
        let span = tracing::trace_span!("poll");
        let _enter = span.enter();

        // Handle pending requests
        while let Poll::Ready(Some((chan, resp))) = self.state.pending_oneshot.poll_next_unpin(cx) {
            if self.admin.respond_final(chan, resp).is_err() {
                tracing::debug!("Client dropped request");
            }
            wake_me_up = true;
        }

        let span = tracing::trace_span!("poll streams");
        let enter = span.enter();
        let mut count = 0;
        while let Poll::Ready(Some((chan, resp))) = self.state.pending_stream.poll_next_unpin(cx) {
            count += 1;
            if let Some(msg) = resp {
                if self.events.respond(chan.clone(), msg).is_err() {
                    if let Some(h) = self.state.stream_handles.remove(&chan) {
                        h.abort();
                    }
                }
            } else {
                self.state.stream_handles.remove(&chan);
                let _ = self.events.finish_response(chan);
            }
            wake_me_up = true;
        }
        tracing::trace!("handled {} stream events", count);
        drop(enter);

        while let Poll::Ready(Some((channel, response))) = self.state.pending_finalise.poll_next_unpin(cx) {
            if let BanyanResponse::Error(ref e) = response {
                tracing::warn!("error in Finalise: {}", e);
            }
            self.banyan.send_response(channel, response).ok();
            wake_me_up = true;
        }

        // This `poll` function is the last in the derived NetworkBehaviour.
        // This means, when interacting with any sub-behaviours here, we have to
        // make sure that they are being polled again. This smells, but it is a
        // limitation or design flaw within libp2p. Not much we can do about it
        // here.
        if wake_me_up {
            cx.waker().wake_by_ref();
        }

        Poll::Pending
    }
}

impl NetworkBehaviourEventProcess<StreamingResponseEvent<AdminProtocol>> for ApiBehaviour {
    fn inject_event(&mut self, event: StreamingResponseEvent<AdminProtocol>) {
        tracing::debug!("Received streaming_response event: {:?}", event);

        match event {
            StreamingResponseEvent::<AdminProtocol>::ReceivedRequest { payload, channel_id } => {
                let peer = channel_id.peer();
                if !self.is_authorized(&peer) {
                    tracing::warn!("Received unauthorized request from {}. Rejecting.", peer);
                    let _ = self.admin.respond_final(
                        channel_id,
                        Err(ActyxOSCode::ERR_UNAUTHORIZED
                            .with_message("Provided key is not authorized to access the API.")),
                    );
                    return;
                }

                self.enqueue(channel_id, payload);
            }
            StreamingResponseEvent::<AdminProtocol>::CancelledRequest { .. } => {
                // all responses are one-shot at the moment, no need to cancel anything ongoing.
            }
            StreamingResponseEvent::<AdminProtocol>::ResponseReceived { .. } => {}
            StreamingResponseEvent::<AdminProtocol>::ResponseFinished { .. } => {}
        }
    }
}

impl NetworkBehaviourEventProcess<StreamingResponseEvent<EventsProtocol>> for ApiBehaviour {
    fn inject_event(&mut self, event: StreamingResponseEvent<EventsProtocol>) {
        tracing::debug!("Received streaming_response event: {:?}", event);

        match event {
            StreamingResponseEvent::<EventsProtocol>::ReceivedRequest { payload, channel_id } => {
                let peer = channel_id.peer();
                if !self.is_authorized(&peer) {
                    tracing::warn!("Received unauthorized request from {}. Rejecting.", peer);
                    let _ = self.admin.respond_final(
                        channel_id,
                        Err(ActyxOSCode::ERR_UNAUTHORIZED
                            .with_message("Provided key is not authorized to access the API.")),
                    );
                    return;
                }

                self.enqueue_events_v2(channel_id, payload);
            }
            StreamingResponseEvent::<EventsProtocol>::CancelledRequest { channel_id, .. } => {
                if let Some(h) = self.state.stream_handles.remove(&channel_id) {
                    h.abort();
                }
            }
            StreamingResponseEvent::<EventsProtocol>::ResponseReceived { .. } => {}
            StreamingResponseEvent::<EventsProtocol>::ResponseFinished { .. } => {}
        }
    }
}

impl NetworkBehaviourEventProcess<RequestResponseEvent<BanyanRequest, BanyanResponse>> for ApiBehaviour {
    fn inject_event(&mut self, event: RequestResponseEvent<BanyanRequest, BanyanResponse>) {
        tracing::debug!("received banyan event");

        match event {
            RequestResponseEvent::Message { peer, message } => {
                tracing::debug!(peer = display(peer), "received {:?}", message);
                match message {
                    RequestResponseMessage::Request { request, channel, .. } => {
                        if !self.is_authorized(&peer) {
                            tracing::warn!("Received unauthorized request from {}. Rejecting.", peer);
                            self.banyan
                                .send_response(
                                    channel,
                                    Err(ActyxOSCode::ERR_UNAUTHORIZED
                                        .with_message("Provided key is not authorized to access the API."))
                                    .into(),
                                )
                                .ok();
                            return;
                        }
                        match request {
                            BanyanRequest::MakeFreshTopic(topic) => {
                                let result = (|| -> anyhow::Result<()> {
                                    remove_old_dbs(self.state.store_dir.as_path(), topic.as_str())
                                        .context("removing old DBs")?;
                                    let storage = StorageServiceStore::new(
                                        StorageService::open(
                                            StorageConfig::new(
                                                Some(self.state.store_dir.join(format!("{}.sqlite", topic))),
                                                None,
                                                10_000,
                                                Duration::from_secs(7200),
                                            ),
                                            swarm::IpfsEmbedExecutor::new(),
                                        )
                                        .context("creating new store DB")?,
                                    );
                                    let forest = swarm::BanyanForest::<swarm::TT, _>::new(storage, Default::default());
                                    tracing::info!("prepared new store DB for upload of topic `{}`", topic);
                                    self.state.banyan_stores.insert(topic, BanyanWriter::new(forest));
                                    Ok(())
                                })();
                                if let Err(ref e) = result {
                                    tracing::warn!("error in MakeFreshTopic: {:#}", e);
                                }
                                self.banyan.send_response(channel, result.into()).ok();
                            }
                            BanyanRequest::AppendEvents(topic, data) => {
                                let result = (|| -> anyhow::Result<()> {
                                    let writer = self
                                        .state
                                        .banyan_stores
                                        .get_mut(&topic)
                                        .ok_or_else(|| anyhow::anyhow!("topic not prepared"))?;
                                    writer.buf.write_all(data.as_slice()).context("feeding decompressor")?;
                                    store_events(writer).context("storing events")?;
                                    Ok(())
                                })();
                                if let Err(ref e) = result {
                                    tracing::warn!("error in AppendEvents: {:#}", e);
                                }
                                self.banyan.send_response(channel, result.into()).ok();
                            }
                            BanyanRequest::Finalise(topic) => {
                                let result = (|| -> anyhow::Result<()> {
                                    let mut writer = self
                                        .state
                                        .banyan_stores
                                        .remove(&topic)
                                        .ok_or_else(|| anyhow::anyhow!("topic not prepared"))?;

                                    writer.buf.flush().context("flushing decompressor")?;
                                    store_events(&mut writer).context("storing final events")?;

                                    if !writer.buf.get_ref().is_empty() {
                                        tracing::warn!(
                                            bytes = writer.buf.get_ref().len(),
                                            "trailing garbage in upload for topic `{}`!",
                                            topic
                                        );
                                    }

                                    finalise_streams(self.state.node_id, writer).context("finalising streams")?;

                                    Ok(())
                                })();
                                if let Err(ref e) = result {
                                    tracing::warn!("error in Finalise: {:#}", e);
                                    self.banyan.send_response(channel, result.into()).ok();
                                    return;
                                }
                                tracing::info!("import completed for topic `{}`", topic);

                                let node_tx = self.state.node_tx.clone();
                                self.state
                                    .pending_finalise
                                    .push(Box::pin(switch_to_dump(node_tx, channel, topic)));
                            }
                            BanyanRequest::Future => {
                                self.banyan
                                    .send_response(channel, BanyanResponse::Error("message from the future".into()))
                                    .ok();
                            }
                        }
                    }
                    RequestResponseMessage::Response { .. } => {}
                }
            }
            RequestResponseEvent::OutboundFailure {
                peer,
                request_id,
                error,
            } => tracing::warn!(
                peer = display(peer),
                request_id = display(request_id),
                error = debug(&error),
                "banyan outbound failure"
            ),
            RequestResponseEvent::InboundFailure {
                peer,
                request_id,
                error,
            } => tracing::warn!(
                peer = display(peer),
                request_id = display(request_id),
                error = debug(&error),
                "banyan inbound failure"
            ),
            RequestResponseEvent::ResponseSent { .. } => {}
        }
    }
}

fn remove_old_dbs(dir: &Path, topic: &str) -> anyhow::Result<()> {
    fn ok(path: PathBuf) -> anyhow::Result<()> {
        match std::fs::remove_file(&path) {
            Ok(_) => {
                tracing::info!("removed {}", path.display());
                Ok(())
            }
            Err(e) => match e.kind() {
                ErrorKind::NotFound => Ok(()),
                _ => Err(e.into()),
            },
        }
    }
    let path = dir.join(format!("{}.sqlite", topic));
    // NotADirectory and IsADirectory are not yet stable, so try to remove the
    // directory first, ignore errors, and notice failure when trying to remove
    // the file (which should be NotFound or success, not a directory).
    std::fs::remove_dir_all(&path)
        .map(|_| tracing::info!("removed {}", path.display()))
        .ok();
    ok(dir.join(format!("{}.sqlite", topic)))?;
    ok(dir.join(format!("{}.sqlite-shm", topic)))?;
    ok(dir.join(format!("{}.sqlite-wal", topic)))?;
    ok(dir.join(format!("{}-index.sqlite", topic)))?;
    ok(dir.join(format!("{}-index.sqlite-shm", topic)))?;
    ok(dir.join(format!("{}-index.sqlite-wal", topic)))?;
    Ok(())
}

fn finalise_streams(node_id: NodeId, mut writer: BanyanWriter) -> Result<(), anyhow::Error> {
    // pack the streams
    writer.txn.pack(&mut writer.own)?;
    writer.txn.pack(&mut writer.other)?;

    // then alias them
    let header = AxTreeHeader::new(writer.own.snapshot().link().unwrap(), writer.lamport);
    let root = writer.txn.writer().put(DagCborCodec.encode(&header)?)?;
    let cid = Cid::from(root);
    let stream_id = node_id.stream(0.into());
    writer
        .txn
        .store()
        .alias(StreamAlias::from(stream_id).as_ref(), Some(&cid))?;
    let header = AxTreeHeader::new(writer.other.snapshot().link().unwrap(), writer.lamport);
    let root = writer.txn.writer().put(DagCborCodec.encode(&header)?)?;
    let cid = Cid::from(root);
    if let Some(node_id) = writer.node_id {
        let stream_id = node_id.stream(4.into());
        writer
            .txn
            .store()
            .alias(StreamAlias::from(stream_id).as_ref(), Some(&cid))?;
    }
    // the SqliteIndexStore will be autofilled with these streams upon restart

    Ok(())
}

async fn switch_to_dump(
    node_tx: Sender<ExternalEvent>,
    channel: ResponseChannel<BanyanResponse>,
    topic: String,
) -> (ResponseChannel<BanyanResponse>, BanyanResponse) {
    let (tx, rx) = oneshot::channel();
    let get_settings = ExternalEvent::SettingsRequest(SettingsRequest::GetSettings {
        scope: "com.actyx".parse().unwrap(),
        no_defaults: false,
        response: tx,
    });
    if node_tx.send(get_settings).is_err() {
        return (channel, BanyanResponse::Error("store closed".into()));
    }
    let mut settings = match rx.await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return (channel, BanyanResponse::Error(e.to_string())),
        Err(e) => return (channel, BanyanResponse::Error(e.to_string())),
    };

    let mut changed = false;
    let ro = settings.pointer_mut("/api/events/readOnly").unwrap();
    if *ro != json!(true) {
        *ro = json!(true);
        changed = true;
    }
    let top = settings.pointer_mut("/swarm/topic").unwrap();
    if *top != json!(topic) {
        *top = json!(topic);
        changed = true;
    }

    if changed {
        let (tx, rx) = oneshot::channel();
        let set_settings = ExternalEvent::SettingsRequest(SettingsRequest::SetSettings {
            scope: "com.actyx".parse().unwrap(),
            json: settings,
            ignore_errors: false,
            response: tx,
        });
        if node_tx.send(set_settings).is_err() {
            return (channel, BanyanResponse::Error("store closed".into()));
        }
        match rx.await {
            Ok(Err(e)) => return (channel, BanyanResponse::Error(e.to_string())),
            Err(e) => return (channel, BanyanResponse::Error(e.to_string())),
            _ => {}
        }
    } else {
        tracing::info!("settings unchanged, restarting store");
        if node_tx
            .send(ExternalEvent::RestartRequest(Store::get_type().into()))
            .is_err()
        {
            return (channel, BanyanResponse::Error("cannot restart store".into()));
        }
    }

    (channel, BanyanResponse::Ok)
}

fn store_events(writer: &mut BanyanWriter) -> anyhow::Result<()> {
    let mut bytes = writer.buf.get_ref().as_slice();
    tracing::debug!("storing event from buffer of {} bytes", bytes.len());
    while let Ok((cbor, rest)) = Cbor::checked_prefix(bytes) {
        tracing::trace!("found data block of {} bytes", cbor.as_slice().len());
        if let Some(node_id) = writer.node_id {
            let (orig_node, app_id, timestamp, tags, payload) =
                decode_dump_frame(cbor).ok_or_else(|| anyhow::anyhow!("malformed event: {}", cbor))?;
            let lamport = writer.lamport.incr();
            writer.lamport = lamport;

            let mut tagset = ScopedTagSet::from(tags);
            tagset.insert(ScopedTag::new(TagScope::Internal, tag!("app_id:") + app_id.as_str()));
            let key = AxKey::new(tagset, lamport, timestamp);

            let stream = if orig_node == node_id {
                &mut writer.own
            } else {
                &mut writer.other
            };

            writer.txn.extend_unpacked(stream, [(key, payload)])?;
            if stream.level() > 500 {
                writer.txn.pack(stream)?;
            }
        } else {
            writer.node_id = Some(
                decode_dump_header(cbor)
                    .ok_or_else(|| anyhow::anyhow!("malformed header: {}", cbor))?
                    .0,
            );
        }
        bytes = rest;
    }
    let consumed = unsafe {
        (bytes as *const _ as *const u8).offset_from(writer.buf.get_ref().as_slice() as *const _ as *const u8)
    };
    tracing::debug!("consumed {} bytes", consumed);
    if consumed > 0 {
        let consumed = consumed as usize;
        let v = writer.buf.get_mut();
        v.as_mut_slice().copy_within(consumed.., 0);
        v.truncate(v.len() - consumed);
    }
    if writer.buf.get_ref().len() > 4000000 {
        anyhow::bail!("upload buffer full");
    }
    Ok(())
}

impl NetworkBehaviourEventProcess<PingEvent> for ApiBehaviour {
    fn inject_event(&mut self, _event: PingEvent) {
        // ignored
    }
}

impl NetworkBehaviourEventProcess<IdentifyEvent> for ApiBehaviour {
    fn inject_event(&mut self, _event: IdentifyEvent) {
        // ignored
    }
}

pub(crate) async fn mk_swarm(
    node_id: NodeId,
    keypair: libp2p::core::identity::Keypair,
    node_tx: Sender<ExternalEvent>,
    bind_to: SocketAddrHelper,
    store_dir: PathBuf,
    store: StoreTx,
    auth_info: Arc<Mutex<NodeApiSettings>>,
) -> anyhow::Result<PeerId> {
    if bind_to.to_multiaddrs().next().is_none() {
        bail!("cannot start node API without any listen addresses");
    }

    let protocol = ApiBehaviour::new(node_id, node_tx, store_dir, store, auth_info, keypair.public());
    let (peer_id, transport) = mk_transport(keypair).await?;

    let mut swarm = SwarmBuilder::new(transport, protocol, peer_id)
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    let mut addrs = swarm.behaviour().state.admin_sockets.new_observer();

    // Trying to bind to `/ip6/::0/tcp/0` (dual-stack) won't work, as
    // rust-libp2p sets `IPV6_V6ONLY` (or the platform equivalent) [0]. This is
    // why we have to to bind to ip4 and ip6 manually.
    // [0] https://github.com/libp2p/rust-libp2p/blob/master/transports/tcp/src/lib.rs#L322
    for addr in bind_to.to_multiaddrs() {
        tracing::debug!("Admin API trying to bind to {}", addr);
        swarm.listen_on(addr.clone()).with_context(|| {
            let port = addr
                .iter()
                .find_map(|x| match x {
                    Protocol::Tcp(p) => Some(p),
                    Protocol::Udp(p) => Some(p),
                    _ => None,
                })
                .unwrap_or_default();
            NodeErrorContext::BindFailed {
                port,
                component: "Admin".into(),
            }
        })?;
    }

    tokio::spawn(SwarmFuture(swarm));

    // check that some addresses were bound
    let mut set = addrs.next().await.ok_or_else(|| anyhow!("address stream died"))?;
    let deadline = Instant::now() + Duration::from_secs(10);
    for addr in bind_to.to_multiaddrs() {
        match addr.into_iter().next() {
            Some(Protocol::Ip4(ip4)) if ip4.is_loopback() || ip4.is_unspecified() => loop {
                if set
                    .iter()
                    .any(|a| matches!(a.iter().next(), Some(Protocol::Ip4(ip)) if ip.is_loopback()))
                {
                    break;
                }
                match timeout_at(deadline, addrs.next()).await {
                    Ok(Some(s)) => set = s,
                    Ok(None) => bail!("address stream died"),
                    Err(_) => bail!("timeout waiting for listeners"),
                };
            },
            Some(Protocol::Ip6(ip6)) if ip6.is_loopback() || ip6.is_unspecified() => loop {
                if set
                    .iter()
                    .any(|a| matches!(a.iter().next(), Some(Protocol::Ip6(ip)) if ip.is_loopback()))
                {
                    break;
                }
                match timeout_at(deadline, addrs.next()).await {
                    Ok(Some(s)) => set = s,
                    Ok(None) => bail!("address stream died"),
                    Err(_) => bail!("timeout waiting for listeners"),
                };
            },
            _ => {}
        }
    }

    Ok(peer_id)
}

type TConnErr = libp2p::core::either::EitherError<
    libp2p::core::either::EitherError<
        libp2p::core::either::EitherError<
            libp2p::core::either::EitherError<
                libp2p::swarm::protocols_handler::ProtocolsHandlerUpgrErr<std::io::Error>,
                libp2p::swarm::protocols_handler::ProtocolsHandlerUpgrErr<std::io::Error>,
            >,
            libp2p::swarm::protocols_handler::ProtocolsHandlerUpgrErr<std::io::Error>,
        >,
        libp2p::ping::handler::PingFailure,
    >,
    std::io::Error,
>;

/// Wrapper object for driving the whole swarm
struct SwarmFuture(WrappedBehaviour);
impl SwarmFuture {
    pub(crate) fn swarm(&mut self) -> &mut WrappedBehaviour {
        &mut self.0
    }

    /// Poll the swarm once
    pub(crate) fn poll_swarm(&mut self, cx: &mut task::Context) -> std::task::Poll<Option<SwarmEvent<(), TConnErr>>> {
        self.swarm().poll_next_unpin(cx)
    }
}

impl Future for SwarmFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context) -> Poll<Self::Output> {
        // poll the swarm until pending
        while let Poll::Ready(Some(event)) = self.poll_swarm(cx) {
            match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    tracing::info!(target: "ADMIN_API_BOUND", "Admin API bound to {}.", address);
                    self.0
                        .behaviour_mut()
                        .state
                        .admin_sockets
                        .transform_mut(|set| set.insert(address));
                }
                SwarmEvent::ListenerError { error, .. } => {
                    tracing::error!("SwarmEvent::ListenerError {}", error)
                }
                SwarmEvent::ListenerClosed {
                    reason: Err(error),
                    addresses,
                    ..
                } => {
                    tracing::error!("SwarmEvent::ListenerClosed {} for {:?}", error, addresses);
                    for addr in addresses {
                        self.0
                            .behaviour_mut()
                            .state
                            .admin_sockets
                            .transform_mut(|set| set.remove(&addr));
                    }
                }
                o => {
                    tracing::debug!("Other swarm event {:?}", o);
                }
            }
        }

        Poll::Pending
    }
}

async fn mk_transport(id_keys: identity::Keypair) -> anyhow::Result<(PeerId, Boxed<(PeerId, StreamMuxerBox)>)> {
    let peer_id = id_keys.public().to_peer_id();
    let transport = swarm::transport::build_transport(id_keys, None, Duration::from_secs(20))
        .await
        .context("Building libp2p transport")?;
    Ok((peer_id, transport))
}
