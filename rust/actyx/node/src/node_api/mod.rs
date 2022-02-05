use crate::{
    components::{
        node_api::NodeApiSettings,
        store::{Store, StoreRequest, StoreTx},
        Component, ComponentRequest,
    },
    formats::ExternalEvent,
    settings::{SettingsRequest, SYSTEM_SCOPE},
    util::trigger_shutdown,
};
use actyx_sdk::{
    app_id,
    service::{QueryResponse, SubscribeMonotonicResponse, SubscribeResponse},
    tag, LamportTimestamp, NodeId, Payload,
};
use anyhow::{anyhow, bail, Context};
use api::EventService;
use ax_futures_util::stream::variable::Variable;
use cbor_data::Cbor;
use crossbeam::channel::Sender;
use crypto::PublicKey;
use formats::NodesRequest;
use futures::{
    channel::mpsc,
    future::{ready, BoxFuture},
    stream::FuturesUnordered,
    task::{self, Poll},
    Future, FutureExt, SinkExt, StreamExt,
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
        NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters, Swarm, SwarmBuilder,
        SwarmEvent,
    },
    Multiaddr, NetworkBehaviour, PeerId,
};
use libp2p_streaming_response::v2::{RequestReceived, StreamingResponse, StreamingResponseConfig};
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
        ActyxOSCode, ActyxOSResult, ActyxOSResultExt, NodeErrorContext, NodesInspectResponse,
    },
    trace_poll::TracePoll,
};
use zstd::stream::write::Decoder;

pub mod formats;

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
    pending_finalise: FuturesUnordered<PendingFinalise>,
    admin_sockets: Variable<BTreeSet<Multiaddr>>,
    banyan_stores: BTreeMap<String, BanyanWriter>,
}

pub struct NoEvent;
impl<T: std::fmt::Debug> From<T> for NoEvent {
    fn from(_: T) -> Self {
        NoEvent
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(poll_method = "poll", out_event = "NoEvent", event_process = true)]
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
            pending_finalise: FuturesUnordered::new(),
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

    fn maybe_add_key(&self, key_id: PublicKey, peer: PeerId) -> Option<BoxFuture<'static, ActyxOSResult<()>>> {
        let mut auth_info = self.state.auth_info.lock();
        if auth_info.authorized_keys.is_empty() {
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
            Some(
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
                .boxed(),
            )
        } else {
            None
        }
    }

    /// The main purpose of this function is to shovel responses from any
    /// pending requests to libp2p.
    fn poll(
        &mut self,
        cx: &mut task::Context,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<NoEvent, <Self as NetworkBehaviour>::ProtocolsHandler>> {
        let mut wake_me_up = false;
        let _span = tracing::trace_span!("poll").entered();

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

impl NetworkBehaviourEventProcess<RequestReceived<AdminProtocol>> for ApiBehaviour {
    fn inject_event(&mut self, mut req: RequestReceived<AdminProtocol>) {
        tracing::debug!("Received streaming_response admin: {:?}", req);
        if !self.is_authorized(&req.peer_id) {
            tracing::warn!("Received unauthorized request from {}. Rejecting.", req.peer_id);
            req.channel
                .try_send(Err(
                    ActyxOSCode::ERR_UNAUTHORIZED.with_message("Provided key is not authorized to access the API.")
                ))
                .ok();
        } else {
            fn respond<T, F>(
                node_tx: Sender<ExternalEvent>,
                mut channel: mpsc::Sender<ActyxOSResult<AdminResponse>>,
                f: F,
                wrap: fn(T) -> AdminResponse,
            ) where
                F: FnOnce(oneshot::Sender<ActyxOSResult<T>>) -> ExternalEvent + Send + 'static,
                T: Send + 'static,
            {
                let (tx, rx) = oneshot::channel();
                node_tx.send(f(tx)).expect("node must keep running");
                tokio::spawn(async move {
                    let result = rx
                        .await
                        .ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "receiving response from node")
                        .unwrap_or_else(|e| Err(e))
                        .map(wrap);
                    channel.feed(result).await.ok();
                });
            }
            match req.request {
                AdminRequest::NodesLs => respond(
                    self.state.node_tx.clone(),
                    req.channel,
                    |tx| ExternalEvent::NodesRequest(NodesRequest::Ls(tx)),
                    AdminResponse::NodesLsResponse,
                ),
                AdminRequest::NodesInspect => {
                    let (tx, rx) = oneshot::channel();
                    let send = self
                        .state
                        .store
                        .send(ComponentRequest::Individual(StoreRequest::NodesInspect(tx)));
                    let admin_addrs = self
                        .state
                        .admin_sockets
                        .get_cloned()
                        .iter()
                        .map(|a| a.to_string())
                        .collect();
                    let mut channel = req.channel;
                    tokio::spawn(
                        async move {
                            send.ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "sending to store")?;
                            let res = rx
                                .await
                                .ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "Error waiting for response")?
                                .ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "Error getting swarm state")?;
                            ActyxOSResult::Ok(AdminResponse::NodesInspectResponse(NodesInspectResponse {
                                peer_id: res.peer_id,
                                swarm_addrs: res.swarm_addrs,
                                announce_addrs: res.announce_addrs,
                                admin_addrs,
                                connections: res.connections,
                                known_peers: res.known_peers,
                            }))
                        }
                        .then(move |res| async move {
                            channel.feed(res).await.ok();
                        }),
                    );
                }
                AdminRequest::NodesShutdown => trigger_shutdown(),
                AdminRequest::SettingsGet { scope, no_defaults } => respond(
                    self.state.node_tx.clone(),
                    req.channel,
                    move |tx| {
                        ExternalEvent::SettingsRequest(SettingsRequest::GetSettings {
                            scope,
                            no_defaults,
                            response: tx,
                        })
                    },
                    AdminResponse::SettingsGetResponse,
                ),
                AdminRequest::SettingsSet {
                    scope,
                    json,
                    ignore_errors,
                } => respond(
                    self.state.node_tx.clone(),
                    req.channel,
                    move |tx| {
                        ExternalEvent::SettingsRequest(SettingsRequest::SetSettings {
                            scope,
                            json,
                            ignore_errors,
                            response: tx,
                        })
                    },
                    AdminResponse::SettingsSetResponse,
                ),
                AdminRequest::SettingsSchema { scope } => respond(
                    self.state.node_tx.clone(),
                    req.channel,
                    |tx| ExternalEvent::SettingsRequest(SettingsRequest::GetSchema { scope, response: tx }),
                    AdminResponse::SettingsSchemaResponse,
                ),
                AdminRequest::SettingsScopes => respond(
                    self.state.node_tx.clone(),
                    req.channel,
                    |tx| ExternalEvent::SettingsRequest(SettingsRequest::GetSchemaScopes { response: tx }),
                    AdminResponse::SettingsScopesResponse,
                ),
                AdminRequest::SettingsUnset { scope } => respond(
                    self.state.node_tx.clone(),
                    req.channel,
                    |tx| ExternalEvent::SettingsRequest(SettingsRequest::UnsetSettings { scope, response: tx }),
                    |_| AdminResponse::SettingsUnsetResponse,
                ),
            };
        }
    }
}

impl NetworkBehaviourEventProcess<RequestReceived<EventsProtocol>> for ApiBehaviour {
    fn inject_event(&mut self, mut req: RequestReceived<EventsProtocol>) {
        tracing::debug!("Received streaming_response event: {:?}", req);
        if !self.is_authorized(&req.peer_id) {
            tracing::warn!("Received unauthorized request from {}. Rejecting.", req.peer_id);
            req.channel
                .try_send(EventsResponse::Error {
                    message: "Provided key is not authorized to access the API.".to_owned(),
                })
                .ok();
        } else {
            let events = self.state.events.clone();
            tokio::spawn(async move {
                match req.request {
                    EventsRequest::Offsets => {
                        req.channel
                            .feed(match events.offsets().await {
                                Ok(o) => EventsResponse::Offsets(o),
                                Err(e) => EventsResponse::Error { message: e.to_string() },
                            })
                            .await?;
                    }
                    EventsRequest::Query(request) => match events.query(app_id!("com.actyx.cli"), request).await {
                        Ok(mut resp) => {
                            tracing::trace!("got response");
                            while let Some(msg) = resp.next().await {
                                tracing::trace!("got message");
                                let item = match msg {
                                    QueryResponse::Event(ev) => {
                                        let span = tracing::trace_span!("ready event");
                                        let _enter = span.enter();
                                        EventsResponse::Event(ev)
                                    }
                                    QueryResponse::Offsets(o) => EventsResponse::OffsetMap { offsets: o.offsets },
                                    QueryResponse::Diagnostic(d) => EventsResponse::Diagnostic(d),
                                    QueryResponse::FutureCompat => continue,
                                };
                                req.channel.feed(item).await?;
                            }
                        }
                        Err(e) => {
                            tracing::trace!("got error");
                            req.channel
                                .feed(EventsResponse::Error { message: e.to_string() })
                                .await?;
                        }
                    },
                    EventsRequest::Subscribe(request) => {
                        match events.subscribe(app_id!("com.actyx.cli"), request).await {
                            Ok(resp) => {
                                req.channel
                                    .send_all(&mut TracePoll::new(
                                        resp.filter_map(move |x| {
                                            tracing::trace!("got subscribe response {:?}", x);
                                            match x {
                                                SubscribeResponse::Event(ev) => {
                                                    let span = tracing::trace_span!("ready event");
                                                    let _enter = span.enter();
                                                    ready(Some(Ok(EventsResponse::Event(ev))))
                                                }
                                                SubscribeResponse::Offsets(o) => {
                                                    ready(Some(Ok(EventsResponse::OffsetMap { offsets: o.offsets })))
                                                }
                                                SubscribeResponse::Diagnostic(d) => {
                                                    ready(Some(Ok(EventsResponse::Diagnostic(d))))
                                                }
                                                SubscribeResponse::FutureCompat => ready(None),
                                            }
                                        }),
                                        "node_api events subscribe",
                                    ))
                                    .await?;
                            }
                            Err(e) => {
                                req.channel
                                    .feed(EventsResponse::Error { message: e.to_string() })
                                    .await?;
                            }
                        }
                    }
                    EventsRequest::SubscribeMonotonic(request) => {
                        match events.subscribe_monotonic(app_id!("com.actyx.cli"), request).await {
                            Ok(resp) => {
                                req.channel
                                    .send_all(&mut TracePoll::new(
                                        resp.filter_map(move |x| {
                                            tracing::trace!("got subscribe response {:?}", x);
                                            match x {
                                                SubscribeMonotonicResponse::Event { event, .. } => {
                                                    let span = tracing::trace_span!("ready event");
                                                    let _enter = span.enter();
                                                    ready(Some(Ok(EventsResponse::Event(event))))
                                                }
                                                SubscribeMonotonicResponse::TimeTravel { .. } => ready(None),
                                                SubscribeMonotonicResponse::Offsets(o) => {
                                                    ready(Some(Ok(EventsResponse::OffsetMap { offsets: o.offsets })))
                                                }
                                                SubscribeMonotonicResponse::Diagnostic(d) => {
                                                    ready(Some(Ok(EventsResponse::Diagnostic(d))))
                                                }
                                                SubscribeMonotonicResponse::FutureCompat => ready(None),
                                            }
                                        }),
                                        "node_api events subscribe",
                                    ))
                                    .await?;
                            }
                            Err(e) => {
                                req.channel
                                    .feed(EventsResponse::Error { message: e.to_string() })
                                    .await?;
                            }
                        }
                    }
                    EventsRequest::Publish(request) => {
                        match events.publish(app_id!("com.actyx.cli"), 0.into(), request).await {
                            Ok(resp) => req.channel.feed(EventsResponse::Publish(resp)).await?,
                            Err(e) => {
                                req.channel
                                    .feed(EventsResponse::Error { message: e.to_string() })
                                    .await?
                            }
                        }
                    }
                }
                ActyxOSResult::Ok(())
            });
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
    let root = writer.txn.writer_mut().put(DagCborCodec.encode(&header)?)?;
    let cid = Cid::from(root);
    let stream_id = node_id.stream(0.into());
    writer
        .txn
        .store()
        .alias(StreamAlias::from(stream_id).as_ref(), Some(&cid))?;
    let header = AxTreeHeader::new(writer.other.snapshot().link().unwrap(), writer.lamport);
    let root = writer.txn.writer_mut().put(DagCborCodec.encode(&header)?)?;
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
    fn inject_event(&mut self, event: IdentifyEvent) {
        match event {
            IdentifyEvent::Received { peer_id, info } => {
                PublicKey::try_from(&info.public_key)
                    .ok()
                    .and_then(|key| self.maybe_add_key(key, peer_id))
                    .map(tokio::spawn);
            }
            IdentifyEvent::Error { peer_id, .. } => {
                // this is an old Actyx v2.0.x node without Identify protocol, where PeerId v0
                // contains the raw public key
                multihash::Multihash::from_bytes(&peer_id.to_bytes())
                    .ok()
                    .filter(|mh| mh.code() == u64::from(multihash::Code::Identity))
                    .and_then(|mh| libp2p::core::identity::PublicKey::from_protobuf_encoding(mh.digest()).ok())
                    .and_then(|pk| match pk {
                        identity::PublicKey::Ed25519(ed) => Some(PublicKey::from(ed)),
                        _ => None,
                    })
                    .and_then(|key| self.maybe_add_key(key, peer_id))
                    .map(tokio::spawn);
            }
            _ => {}
        }
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
        swarm
            .listen_on(addr.clone())
            .with_context(|| NodeErrorContext::BindFailed {
                addr,
                component: "Admin".into(),
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
                libp2p_streaming_response::v2::ProtocolError,
                libp2p_streaming_response::v2::ProtocolError,
            >,
            libp2p::swarm::protocols_handler::ProtocolsHandlerUpgrErr<std::io::Error>,
        >,
        libp2p::ping::PingFailure,
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
    pub(crate) fn poll_swarm(
        &mut self,
        cx: &mut task::Context,
    ) -> std::task::Poll<Option<SwarmEvent<NoEvent, TConnErr>>> {
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
                SwarmEvent::ListenerClosed { reason, addresses, .. } => {
                    tracing::error!(reason = ?&reason, addrs = ?&addresses, "listener closed");
                }
                SwarmEvent::Behaviour(_) => {}
                SwarmEvent::ConnectionEstablished { endpoint, .. } => {
                    tracing::debug!(endpoint = ?&endpoint, "connection established");
                }
                SwarmEvent::ConnectionClosed { endpoint, .. } => {
                    tracing::debug!(endpoint = ?&endpoint, "connection closed");
                }
                SwarmEvent::IncomingConnection { .. } => {}
                SwarmEvent::IncomingConnectionError {
                    local_addr,
                    send_back_addr,
                    error,
                } => {
                    tracing::warn!(local = %&local_addr, remote = %&send_back_addr, error = %&error, "incoming connection failure");
                }
                SwarmEvent::OutgoingConnectionError { .. } => {}
                SwarmEvent::BannedPeer { .. } => {}
                SwarmEvent::ExpiredListenAddr { address, .. } => {
                    tracing::info!("unbound from listen address {}", address);
                }
                SwarmEvent::Dialing(_) => {}
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
