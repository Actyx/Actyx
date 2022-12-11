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
    future::{poll_fn, select_all, BoxFuture},
    stream::FuturesUnordered,
    FutureExt, SinkExt, StreamExt,
};
use libipld::{cbor::DagCborCodec, codec::Codec, Cid};
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed},
    identify, identity,
    multiaddr::Protocol,
    ping,
    request_response::{
        ProtocolSupport, RequestResponse, RequestResponseConfig, RequestResponseEvent, RequestResponseMessage,
        ResponseChannel,
    },
    swarm::{keep_alive, NetworkBehaviour, Swarm, SwarmBuilder, SwarmEvent},
    Multiaddr, PeerId,
};
use libp2p_streaming_response::{RequestReceived, StreamingResponse, StreamingResponseConfig};
use parking_lot::Mutex;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    sync::Arc,
    task::Poll,
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
use util::formats::{
    admin_protocol::{AdminProtocol, AdminRequest, AdminResponse},
    banyan_protocol::{
        decode_dump_frame, decode_dump_header, BanyanProtocol, BanyanProtocolName, BanyanRequest, BanyanResponse,
    },
    events_protocol::{EventsProtocol, EventsRequest, EventsResponse},
    ActyxOSCode, ActyxOSResult, ActyxOSResultExt, NodeErrorContext, NodesInspectResponse,
};
use util::{version::NodeVersion, SocketAddrHelper};
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

#[derive(NetworkBehaviour)]
pub struct ApiBehaviour {
    admin: StreamingResponse<AdminProtocol>,
    events: StreamingResponse<EventsProtocol>,
    banyan: RequestResponse<BanyanProtocol>,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
    keep_alive: keep_alive::Behaviour,
}

impl ApiBehaviour {
    fn new(
        node_id: NodeId,
        node_tx: Sender<ExternalEvent>,
        store_dir: PathBuf,
        store: StoreTx,
        auth_info: Arc<Mutex<NodeApiSettings>>,
        local_public_key: libp2p::core::PublicKey,
    ) -> (Self, State) {
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
        let ret = Self {
            ping: ping::Behaviour::new(ping::Config::new()),
            admin: StreamingResponse::new(StreamingResponseConfig::default()),
            banyan: RequestResponse::new(
                BanyanProtocol::default(),
                [(BanyanProtocolName, ProtocolSupport::Inbound)],
                request_response_config,
            ),
            events: StreamingResponse::new(StreamingResponseConfig::default()),
            identify: identify::Behaviour::new(
                identify::Config::new(format!("Actyx-{}", NodeVersion::get()), local_public_key)
                    .with_initial_delay(Duration::ZERO),
            ),
            keep_alive: keep_alive::Behaviour,
        };
        (ret, state)
    }
}

impl State {
    /// Checks whether `peer` is authorized to use this API. If there are no
    /// authorized keys, any connected peer is authorized.
    fn is_authorized(&self, peer: &PeerId) -> bool {
        let g = self.auth_info.lock();
        g.authorized_keys.is_empty() || g.authorized_keys.contains(peer)
    }

    fn maybe_add_key(&self, key_id: PublicKey, peer: PeerId) -> Option<BoxFuture<'static, ActyxOSResult<()>>> {
        let mut auth_info = self.auth_info.lock();
        if auth_info.authorized_keys.is_empty() {
            tracing::debug!("Adding {} (peer {}) to authorized users", key_id, peer);
            // Directly add the peer. This will be overridden as soon as the settings round
            // tripped.
            auth_info.authorized_keys.push(peer);
            drop(auth_info);
            let (tx, rx) = tokio::sync::oneshot::channel();
            self.node_tx
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
}

#[derive(Debug)]
enum MyEvent {
    Swarm(Option<SwarmEvent<ApiBehaviourEvent, TConnErr>>),
    Finalise(Option<(ResponseChannel<BanyanResponse>, BanyanResponse)>),
}

async fn poll_swarm(mut swarm: Swarm<ApiBehaviour>, mut state: State) {
    loop {
        tracing::trace!("next poll loop");
        let s1 = poll_fn(|cx| {
            tracing::trace!("polling swarm ({:?})", std::thread::current().id());
            swarm.poll_next_unpin(cx).map(MyEvent::Swarm)
        });
        let State { pending_finalise, .. } = &mut state;
        let s2 = poll_fn(|cx| {
            if pending_finalise.is_empty() {
                Poll::Pending
            } else {
                pending_finalise.poll_next_unpin(cx).map(MyEvent::Finalise)
            }
        });
        let all = [s1.left_future(), s2.right_future()];
        let event = select_all(all).await.0;
        tracing::trace!(?event, "got event");
        match event {
            MyEvent::Swarm(Some(event)) => match event {
                SwarmEvent::Behaviour(event) => match event {
                    ApiBehaviourEvent::Admin(event) => inject_admin_event(&mut state, event),
                    ApiBehaviourEvent::Events(event) => inject_events_event(&mut state, event),
                    ApiBehaviourEvent::Banyan(event) => inject_banyan_event(&mut state, swarm.behaviour_mut(), event),
                    ApiBehaviourEvent::Ping(_x) => {}
                    ApiBehaviourEvent::Identify(event) => inject_identify_event(&mut state, event),
                    ApiBehaviourEvent::KeepAlive(v) => void::unreachable(v),
                },
                SwarmEvent::NewListenAddr { address, .. } => {
                    tracing::info!(target: "ADMIN_API_BOUND", "Admin API bound to {}.", address);
                    state.admin_sockets.transform_mut(|set| set.insert(address));
                }
                SwarmEvent::ExpiredListenAddr { address, .. } => {
                    tracing::info!("unbound from listen address {}", address);
                    state.admin_sockets.transform_mut(|set| set.remove(&address));
                }
                SwarmEvent::ListenerError { error, .. } => {
                    tracing::error!("SwarmEvent::ListenerError {}", error)
                }
                SwarmEvent::ListenerClosed { reason, addresses, .. } => {
                    tracing::error!(reason = ?&reason, addrs = ?&addresses, "listener closed");
                    state.admin_sockets.transform_mut(|set| {
                        for addr in addresses {
                            set.remove(&addr);
                        }
                        true
                    });
                }
                SwarmEvent::ConnectionEstablished { endpoint, .. } => {
                    tracing::debug!(endpoint = ?&endpoint, "connection established");
                }
                SwarmEvent::ConnectionClosed { endpoint, .. } => {
                    tracing::debug!(endpoint = ?&endpoint, "connection closed");
                }
                SwarmEvent::IncomingConnectionError {
                    local_addr,
                    send_back_addr,
                    error,
                } => {
                    tracing::warn!(local = %&local_addr, remote = %&send_back_addr, error = %&error, "incoming connection failure");
                }
                SwarmEvent::IncomingConnection { .. } => {}
                SwarmEvent::OutgoingConnectionError { .. } => {}
                SwarmEvent::BannedPeer { .. } => {}
                SwarmEvent::Dialing(_) => {}
            },
            MyEvent::Finalise(Some((channel, response))) => {
                if let BanyanResponse::Error(err) = &response {
                    tracing::warn!("error in Finalise command: {}", err);
                    swarm.behaviour_mut().banyan.send_response(channel, response).ok();
                }
            }
            _ => {}
        }
    }
}

fn inject_admin_event(state: &mut State, event: RequestReceived<AdminProtocol>) {
    let RequestReceived {
        peer_id,
        connection: _,
        request,
        mut channel,
    } = event;
    tracing::debug!("Received streaming_response admin: {:?}", request);
    if !state.is_authorized(&peer_id) {
        tracing::warn!("Received unauthorized request from {}. Rejecting.", peer_id);
        channel
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
        match request {
            AdminRequest::NodesLs => respond(
                state.node_tx.clone(),
                channel,
                |tx| ExternalEvent::NodesRequest(NodesRequest::Ls(tx)),
                AdminResponse::NodesLsResponse,
            ),
            AdminRequest::NodesInspect => {
                let (tx, rx) = oneshot::channel();
                let send = state
                    .store
                    .send(ComponentRequest::Individual(StoreRequest::NodesInspect(tx)));
                let admin_addrs = state.admin_sockets.get_cloned().iter().map(|a| a.to_string()).collect();
                let mut channel = channel;
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
            AdminRequest::NodesShutdown => trigger_shutdown(true),
            AdminRequest::SettingsGet { scope, no_defaults } => respond(
                state.node_tx.clone(),
                channel,
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
                state.node_tx.clone(),
                channel,
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
                state.node_tx.clone(),
                channel,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::GetSchema { scope, response: tx }),
                AdminResponse::SettingsSchemaResponse,
            ),
            AdminRequest::SettingsScopes => respond(
                state.node_tx.clone(),
                channel,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::GetSchemaScopes { response: tx }),
                AdminResponse::SettingsScopesResponse,
            ),
            AdminRequest::SettingsUnset { scope } => respond(
                state.node_tx.clone(),
                channel,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::UnsetSettings { scope, response: tx }),
                |_| AdminResponse::SettingsUnsetResponse,
            ),
        };
    }
}

fn inject_events_event(state: &mut State, event: RequestReceived<EventsProtocol>) {
    let RequestReceived {
        peer_id,
        connection: _,
        request,
        mut channel,
    } = event;
    tracing::debug!("Received streaming_response event: {:?}", request);
    if !state.is_authorized(&peer_id) {
        tracing::warn!("Received unauthorized request from {}. Rejecting.", peer_id);
        tokio::spawn(async move {
            channel
                .feed(EventsResponse::Error {
                    message: "Provided key is not authorized to access the API.".to_owned(),
                })
                .await
        });
    } else {
        let events = state.events.clone();
        tokio::spawn(async move {
            match request {
                EventsRequest::Offsets => {
                    channel
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
                                QueryResponse::Event(ev) => EventsResponse::Event(ev),
                                QueryResponse::Offsets(o) => EventsResponse::OffsetMap { offsets: o.offsets },
                                QueryResponse::Diagnostic(d) => EventsResponse::Diagnostic(d),
                                QueryResponse::FutureCompat => continue,
                            };
                            channel.feed(item).await?;
                        }
                    }
                    Err(e) => {
                        tracing::trace!("got error");
                        channel.feed(EventsResponse::Error { message: e.to_string() }).await?;
                    }
                },
                EventsRequest::Subscribe(request) => match events.subscribe(app_id!("com.actyx.cli"), request).await {
                    Ok(mut resp) => {
                        tracing::trace!("got response");
                        while let Some(msg) = resp.next().await {
                            tracing::trace!("got message");
                            let item = match msg {
                                SubscribeResponse::Event(ev) => EventsResponse::Event(ev),
                                SubscribeResponse::AntiEvent(ev) => EventsResponse::AntiEvent(ev),
                                SubscribeResponse::Offsets(o) => EventsResponse::OffsetMap { offsets: o.offsets },
                                SubscribeResponse::Diagnostic(d) => EventsResponse::Diagnostic(d),
                                SubscribeResponse::FutureCompat => continue,
                            };
                            channel.feed(item).await?;
                        }
                    }
                    Err(e) => {
                        channel.feed(EventsResponse::Error { message: e.to_string() }).await?;
                    }
                },
                EventsRequest::SubscribeMonotonic(request) => {
                    match events.subscribe_monotonic(app_id!("com.actyx.cli"), request).await {
                        Ok(mut resp) => {
                            tracing::trace!("got response");
                            while let Some(msg) = resp.next().await {
                                tracing::trace!("got message");
                                let item = match msg {
                                    SubscribeMonotonicResponse::Event { event, .. } => EventsResponse::Event(event),
                                    SubscribeMonotonicResponse::Offsets(o) => {
                                        EventsResponse::OffsetMap { offsets: o.offsets }
                                    }
                                    SubscribeMonotonicResponse::Diagnostic(d) => EventsResponse::Diagnostic(d),
                                    SubscribeMonotonicResponse::FutureCompat => continue,
                                    SubscribeMonotonicResponse::TimeTravel { .. } => continue,
                                };
                                channel.feed(item).await?;
                            }
                        }
                        Err(e) => {
                            channel.feed(EventsResponse::Error { message: e.to_string() }).await?;
                        }
                    }
                }
                EventsRequest::Publish(request) => {
                    match events.publish(app_id!("com.actyx.cli"), 0.into(), request).await {
                        Ok(resp) => channel.feed(EventsResponse::Publish(resp)).await?,
                        Err(e) => channel.feed(EventsResponse::Error { message: e.to_string() }).await?,
                    }
                }
            }
            ActyxOSResult::Ok(())
        });
    }
}

fn inject_banyan_event(
    state: &mut State,
    swarm: &mut ApiBehaviour,
    event: RequestResponseEvent<BanyanRequest, BanyanResponse>,
) {
    tracing::debug!("received banyan event");

    match event {
        RequestResponseEvent::Message { peer, message } => {
            tracing::debug!(peer = display(peer), "received {:?}", message);
            match message {
                RequestResponseMessage::Request { request, channel, .. } => {
                    if !state.is_authorized(&peer) {
                        tracing::warn!("Received unauthorized request from {}. Rejecting.", peer);
                        swarm
                            .banyan
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
                                remove_old_dbs(state.store_dir.as_path(), topic.as_str())
                                    .context("removing old DBs")?;
                                let storage = StorageServiceStore::new(
                                    StorageService::open(
                                        StorageConfig::new(
                                            Some(state.store_dir.join(format!("{}.sqlite", topic))),
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
                                state.banyan_stores.insert(topic, BanyanWriter::new(forest));
                                Ok(())
                            })();
                            if let Err(ref e) = result {
                                tracing::warn!("error in MakeFreshTopic: {:#}", e);
                            }
                            swarm.banyan.send_response(channel, result.into()).ok();
                        }
                        BanyanRequest::AppendEvents(topic, data) => {
                            let result = (|| -> anyhow::Result<()> {
                                let writer = state
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
                            swarm.banyan.send_response(channel, result.into()).ok();
                        }
                        BanyanRequest::Finalise(topic) => {
                            let result = (|| -> anyhow::Result<()> {
                                let mut writer = state
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

                                finalise_streams(state.node_id, writer).context("finalising streams")?;

                                Ok(())
                            })();
                            if let Err(ref e) = result {
                                tracing::warn!("error in Finalise: {:#}", e);
                                swarm.banyan.send_response(channel, result.into()).ok();
                                return;
                            }
                            tracing::info!("import completed for topic `{}`", topic);

                            let node_tx = state.node_tx.clone();
                            state
                                .pending_finalise
                                .push(Box::pin(switch_to_dump(node_tx, channel, topic)));
                        }
                        BanyanRequest::Future => {
                            swarm
                                .banyan
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

fn inject_identify_event(state: &mut State, event: identify::Event) {
    match event {
        identify::Event::Received { peer_id, info } => {
            PublicKey::try_from(&info.public_key)
                .ok()
                .and_then(|key| state.maybe_add_key(key, peer_id))
                .map(tokio::spawn);
        }
        identify::Event::Error { peer_id, .. } => {
            // this is an old Actyx v2.0.x node without Identify protocol, where PeerId v0
            // contains the raw public key
            multihash::Multihash::from_bytes(&peer_id.to_bytes())
                .ok()
                .filter(|mh| mh.code() == u64::from(multihash::Code::Identity))
                .and_then(|mh| libp2p::core::identity::PublicKey::from_protobuf_encoding(mh.digest()).ok())
                .map(|pk| match pk {
                    identity::PublicKey::Ed25519(ed) => PublicKey::from(ed),
                })
                .and_then(|key| state.maybe_add_key(key, peer_id))
                .map(tokio::spawn);
        }
        _ => {}
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

    let (protocol, state) = ApiBehaviour::new(node_id, node_tx, store_dir, store, auth_info, keypair.public());
    let (peer_id, transport) = mk_transport(keypair).await?;

    let mut swarm = SwarmBuilder::with_tokio_executor(transport, protocol, peer_id).build();

    let mut addrs = state.admin_sockets.new_observer();

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

    tokio::spawn(poll_swarm(swarm, state));

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

type TConnErr = <<<ApiBehaviour as NetworkBehaviour>::ConnectionHandler as libp2p::swarm::IntoConnectionHandler>::Handler as libp2p::swarm::ConnectionHandler>::Error;

async fn mk_transport(id_keys: identity::Keypair) -> anyhow::Result<(PeerId, Boxed<(PeerId, StreamMuxerBox)>)> {
    let peer_id = id_keys.public().to_peer_id();
    let transport = swarm::transport::build_transport(id_keys, None, Duration::from_secs(20))
        .await
        .context("Building libp2p transport")?;
    Ok((peer_id, transport))
}
