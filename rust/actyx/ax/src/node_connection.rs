use crate::{cmd::Authority, private_key::AxPrivateKey};
use actyx_sdk::{
    service::{Diagnostic, EventResponse},
    NodeId, Payload,
};
use anyhow::anyhow;
use crypto::PublicKey;
use derive_more::From;
use futures::{
    channel::mpsc::{self, channel, Receiver, Sender, TrySendError},
    future::{poll_fn, ready},
    stream::BoxStream,
    Future, FutureExt, SinkExt, StreamExt,
};
use libp2p::{
    identify,
    multiaddr::Protocol,
    ping,
    request_response::{
        ProtocolSupport, RequestId, RequestResponse, RequestResponseConfig, RequestResponseEvent,
        RequestResponseMessage,
    },
    swarm::{dial_opts::DialOpts, DialError, NetworkBehaviour, SwarmBuilder, SwarmEvent},
    Multiaddr, PeerId,
};
use libp2p_streaming_response::{RequestReceived, Response, StreamingResponse, StreamingResponseConfig};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    convert::{TryFrom, TryInto},
    error::Error,
    fmt::{Debug, Write},
    task::Poll,
    time::Duration,
};
use swarm::transport::build_transport;
use tokio::sync::mpsc::UnboundedSender;
use util::{
    formats::{
        banyan_protocol::{BanyanProtocol, BanyanProtocolName, BanyanRequest, BanyanResponse},
        events_protocol::{EventsProtocol, EventsRequest, EventsResponse},
        ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt, AdminProtocol, AdminRequest, AdminResponse,
    },
    version::NodeVersion,
};

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "OutEvent")]
struct Behaviour {
    admin: StreamingResponse<AdminProtocol>,
    events: StreamingResponse<EventsProtocol>,
    banyan: RequestResponse<BanyanProtocol>,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}

#[derive(Debug, From)]
#[allow(clippy::large_enum_variant)]
enum OutEvent {
    Admin(RequestReceived<AdminProtocol>),
    Events(RequestReceived<EventsProtocol>),
    Banyan(RequestResponseEvent<BanyanRequest, BanyanResponse>),
    Ping(ping::Event),
    Identify(identify::Event),
}

pub enum Task {
    Connect(Authority, Sender<ActyxOSResult<PeerId>>),
    Admin(PeerId, AdminRequest, Sender<ActyxOSResult<AdminResponse>>),
    Events(PeerId, EventsRequest, Sender<ActyxOSResult<EventsResponse>>),
    Banyan(PeerId, BanyanRequest, Sender<ActyxOSResult<BanyanResponse>>),
    NodeId(PeerId, Sender<ActyxOSResult<(NodeId, NodeVersion)>>),
    #[allow(dead_code)]
    OnDisconnect(UnboundedSender<PeerId>),
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(arg0, _arg1) => f.debug_tuple("Connect").field(arg0).finish(),
            Self::Admin(p, arg0, _arg1) => f.debug_tuple("Admin").field(p).field(arg0).finish(),
            Self::Events(p, arg0, _arg1) => f.debug_tuple("Events").field(p).field(arg0).finish(),
            Self::Banyan(p, arg0, _arg1) => f.debug_tuple("Banyan").field(p).field(arg0).finish(),
            Self::NodeId(p, _arg0) => f.debug_tuple("NodeId").field(p).finish(),
            Self::OnDisconnect(_) => f.debug_tuple("OnDisconnect").finish(),
        }
    }
}

pub async fn mk_swarm(key: AxPrivateKey) -> ActyxOSResult<(impl Future<Output = ()> + Send + 'static, Sender<Task>)> {
    let (tx, mut rx) = channel(10);

    let key_pair = key.to_libp2p_pair();
    let public_key = key_pair.public();
    let local_peer_id = public_key.to_peer_id();
    let transport = build_transport(key_pair, None, Duration::from_secs(20))
        .await
        .ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "cannot build network transport")?;
    let behaviour = Behaviour {
        admin: StreamingResponse::new(StreamingResponseConfig::default().with_request_timeout(Duration::from_secs(20))),
        events: StreamingResponse::new(
            StreamingResponseConfig::default()
                .with_request_timeout(Duration::from_secs(20))
                .with_keep_alive(true),
        ),
        banyan: RequestResponse::new(
            BanyanProtocol::default(),
            [(BanyanProtocolName, ProtocolSupport::Outbound)],
            {
                let mut cfg = RequestResponseConfig::default();
                cfg.set_request_timeout(Duration::from_secs(120));
                cfg
            },
        ),
        ping: ping::Behaviour::new(ping::Config::new()),
        identify: identify::Behaviour::new(
            identify::Config::new("Actyx".to_owned(), public_key).with_initial_delay(Duration::ZERO),
        ),
    };
    let mut swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build();

    let mut banyan_channels = HashMap::<(PeerId, RequestId), Sender<ActyxOSResult<BanyanResponse>>>::new();
    let mut connects = HashMap::<Multiaddr, Vec<Sender<ActyxOSResult<PeerId>>>>::new();
    let mut awaiting_info = HashMap::<PeerId, Vec<Sender<ActyxOSResult<PeerId>>>>::new();
    let mut infos = HashMap::<PeerId, (Option<PublicKey>, BTreeSet<String>, Option<NodeVersion>)>::new();
    let mut disconnects = Vec::<UnboundedSender<PeerId>>::new();
    let task = poll_fn(move |cx| {
        loop {
            tracing::debug!("polling swarm");
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(ev)) => {
                    tracing::info!("swarm event: {:?}", ev);
                    match ev {
                        SwarmEvent::Behaviour(OutEvent::Banyan(RequestResponseEvent::Message {
                            message: RequestResponseMessage::Response { request_id, response },
                            peer,
                        })) => {
                            if let Some(mut channel) = banyan_channels.remove(&(peer, request_id)) {
                                if channel.try_send(Ok(response)).is_err() {
                                    tracing::warn!("dropping banyan response");
                                }
                            } else {
                                tracing::warn!("got response for unknown ID {}", request_id);
                            }
                        }
                        SwarmEvent::Behaviour(OutEvent::Identify(identify::Event::Received { info, peer_id })) => {
                            let peer_public_key = PublicKey::try_from(&info.public_key).ok();
                            let e = infos.entry(peer_id).or_default();
                            e.0 = e.0.or(peer_public_key);
                            e.1.clear();
                            e.1.extend(info.protocols);
                            e.2 = info
                                .protocol_version
                                .strip_prefix("Actyx-")
                                .and_then(|s| s.parse().ok());
                            for mut sender in awaiting_info.remove(&peer_id).unwrap_or_default() {
                                sender.try_send(Ok(peer_id)).log();
                            }
                        }
                        SwarmEvent::Behaviour(OutEvent::Identify(identify::Event::Error { peer_id, .. })) => {
                            // Actyx v2.0.x didn’t have the identify protocol
                            let mut protos = BTreeSet::new();
                            protos.insert("/actyx/admin/1.0.0".to_owned());
                            infos.entry(peer_id).or_insert_with(|| (None, protos, None));
                            for mut sender in awaiting_info.remove(&peer_id).unwrap_or_default() {
                                sender.try_send(Ok(peer_id)).log();
                            }
                        }
                        SwarmEvent::OutgoingConnectionError { error, .. } => {
                            tracing::error!("connection error: {}", error);
                            match error {
                                DialError::ConnectionLimit(_) => {
                                    for (_, sender) in connects.drain() {
                                        let error = Err(ActyxOSCode::ERR_IO.with_message("connection limit reached"));
                                        for mut sender in sender {
                                            sender.try_send(error.clone()).log();
                                        }
                                    }
                                }
                                DialError::WrongPeerId { obtained, endpoint } => {
                                    let addr = endpoint.get_remote_address();
                                    let error = ActyxOSCode::ERR_NODE_UNREACHABLE
                                        .with_message(format!("wrong PeerId {}", obtained));
                                    conn_errors(&mut connects, addr, error);
                                }
                                DialError::Transport(e) => {
                                    for (addr, error) in e {
                                        let mut s = error.to_string();
                                        if let Some(cause) = error.source() {
                                            write!(&mut s, "{}", cause).ok();
                                        }
                                        let error = ActyxOSCode::ERR_NODE_UNREACHABLE.with_message(s);
                                        conn_errors(&mut connects, &addr, error);
                                    }
                                }
                                _ => {}
                            }
                        }
                        SwarmEvent::ConnectionEstablished {
                            peer_id,
                            endpoint,
                            concurrent_dial_errors,
                            ..
                        } => {
                            for (addr, error) in concurrent_dial_errors.unwrap_or_default() {
                                tracing::error!("error dialling {}: {}", addr, error);
                                let error =
                                    ActyxOSCode::ERR_IO.with_message(format!("error dialling {}: {}", addr, error));
                                conn_errors(&mut connects, &addr, error);
                            }
                            let conn = connects.remove(endpoint.get_remote_address()).unwrap_or_default();
                            if infos.contains_key(&peer_id) {
                                for mut sender in conn {
                                    sender.try_send(Ok(peer_id)).log();
                                }
                            } else {
                                awaiting_info.entry(peer_id).or_default().extend(conn);
                            }
                        }
                        SwarmEvent::ConnectionClosed {
                            peer_id,
                            num_established,
                            cause: Some(error),
                            ..
                        } if num_established == 0 => {
                            tracing::warn!("peer disconnected: {}", error);
                            banyan_channels.retain(|(peer, _req), sender| {
                                if *peer == peer_id {
                                    sender
                                        .try_send(Err(ActyxOSCode::ERR_NODE_UNREACHABLE
                                            .with_message("connection closed during request")))
                                        .log();
                                    false
                                } else {
                                    true
                                }
                            });
                            disconnects.retain(|tx| tx.send(peer_id).is_ok());
                        }
                        _ => {}
                    }
                }
                Poll::Ready(None) => return Poll::Ready(Err(anyhow!("swarm stopped"))),
                Poll::Pending => break,
            }
        }
        loop {
            tracing::debug!("polling tasks");
            match rx.poll_next_unpin(cx) {
                Poll::Ready(Some(task)) => {
                    tracing::debug!("task: {:?}", task);
                    match task {
                        Task::Connect(request, mut channel) => {
                            if request.addrs.is_empty() {
                                let e = ActyxOSCode::ERR_INVALID_INPUT
                                    .with_message(format!("no addresses found for `{}`", request.original));
                                channel.try_send(Err(e)).log();
                                continue;
                            }
                            let mut errors = Vec::new();
                            let mut successes = 0;
                            for addr in request.addrs {
                                let opts = if let Some(Protocol::P2p(peer)) = addr.iter().last() {
                                    DialOpts::peer_id(peer.try_into().map_err(|_| {
                                        ActyxOSCode::ERR_INVALID_INPUT
                                            .with_message(format!("`{}` is not a valid PeerId", Protocol::P2p(peer)))
                                    })?)
                                    .addresses(vec![addr.clone()])
                                    .build()
                                } else {
                                    DialOpts::unknown_peer_id().address(addr.clone()).build()
                                };
                                if let Err(e) = swarm.dial(opts) {
                                    tracing::error!("cannot dial `{}`: {}", addr, e);
                                    errors.push(ActyxOSCode::ERR_IO.with_message(format!("cannot dial `{}`", addr)));
                                } else {
                                    successes += 1;
                                    connects.entry(addr).or_default().push(channel.clone())
                                }
                            }
                            if successes == 0 {
                                for error in errors {
                                    channel.try_send(Err(error)).log();
                                }
                            }
                        }
                        Task::Admin(peer_id, request, mut channel) => {
                            if unsupported_proto(
                                infos.get(&peer_id),
                                &["/actyx/admin/1.0.0", "/actyx/admin/1.1"],
                                &mut channel,
                            ) {
                                continue;
                            }
                            let (tx, rx) = mpsc::channel(128);
                            swarm.behaviour_mut().admin.request(peer_id, request, tx);
                            forward_stream(rx, channel, |ev| ev);
                        }
                        Task::Events(peer_id, request, mut channel) => {
                            if unsupported_proto(
                                infos.get(&peer_id),
                                &["/actyx/events/v2", "/actyx/events/v3"],
                                &mut channel,
                            ) {
                                continue;
                            }
                            // in case the node uses v1 or v2 protocol there is no back-pressure, so avoid dropping
                            let has_back_pressure = infos
                                .get(&peer_id)
                                .map(|(_, protos, _)| protos.contains("/actyx/events/v3"))
                                .unwrap_or_default();
                            let buffer = if has_back_pressure { 128 } else { 100000 };
                            let (tx, rx) = mpsc::channel(buffer);
                            swarm.behaviour_mut().events.request(peer_id, request, tx);
                            forward_stream(rx, channel, Ok);
                        }
                        Task::Banyan(peer_id, request, mut channel) => {
                            if unsupported_proto(infos.get(&peer_id), &["/actyx/banyan/create"], &mut channel) {
                                continue;
                            }
                            let id = swarm.behaviour_mut().banyan.send_request(&peer_id, request);
                            banyan_channels.insert((peer_id, id), channel);
                        }
                        Task::NodeId(peer_id, mut channel) => {
                            let (key, _protos, version) = match infos.get(&peer_id) {
                                Some(x) => x,
                                None => {
                                    channel
                                        .try_send(Err(ActyxOSCode::ERR_NODE_UNREACHABLE.with_message("not connected")))
                                        .log();
                                    continue;
                                }
                            };
                            let node_id = key.map(NodeId::from).zip(version.clone()).ok_or_else(|| {
                                ActyxOSError::new(
                                    ActyxOSCode::ERR_UNSUPPORTED,
                                    "remote node does not provide a public key",
                                )
                            });
                            channel.try_send(node_id).log();
                        }
                        Task::OnDisconnect(tx) => disconnects.push(tx),
                    }
                    // need to poll the swarm again to process the request
                    cx.waker().wake_by_ref()
                }
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => break,
            }
        }
        tracing::debug!("sleeping");
        Poll::Pending
    })
    .map(|res| {
        tracing::debug!("event loop ended");
        if let Err(e) = res {
            tracing::error!("event loop error: {}", e);
        }
    });

    Ok((task, tx))
}

fn conn_errors(
    connects: &mut HashMap<Multiaddr, Vec<Sender<Result<PeerId, ActyxOSError>>>>,
    addr: &Multiaddr,
    error: ActyxOSError,
) {
    connects.retain(|a, s| {
        if is_prefix(addr, a) {
            for sender in s {
                sender.try_send(Err(error.clone())).log();
            }
            false
        } else {
            true
        }
    });
}

fn is_prefix(left: &Multiaddr, right: &Multiaddr) -> bool {
    left.iter()
        .zip(right.iter().chain(std::iter::repeat(Protocol::Memory(0))))
        .all(|(l, r)| l == r)
}

fn forward_stream<T: Send + 'static, U: Send + 'static>(
    mut rx: Receiver<Response<T>>,
    mut tx: Sender<ActyxOSResult<U>>,
    transform: impl Fn(T) -> ActyxOSResult<U> + Send + 'static,
) {
    tokio::spawn(async move {
        while let Some(ev) = rx.next().await {
            match ev {
                Response::Msg(ev) => {
                    let ev = transform(ev);
                    if let Err(e) = tx.feed(ev).await {
                        tracing::error!("cannot transfer result: {}", e);
                        return;
                    }
                }
                Response::Error(e) => {
                    if let Err(ee) = tx.feed(Err(ActyxOSCode::ERR_IO.with_message(e.to_string()))).await {
                        tracing::error!("cannot transfer error {}: {}", e, ee);
                    }
                    return;
                }
                Response::Finished => return,
            };
        }
        tracing::error!("response stream ended abruptly");
    });
}

fn unsupported_proto<T: Debug>(
    infos: Option<&(Option<PublicKey>, BTreeSet<String>, Option<NodeVersion>)>,
    choices: &[&str],
    tx: &mut Sender<ActyxOSResult<T>>,
) -> bool {
    let (_key, protos, _version) = if let Some(i) = infos {
        i
    } else {
        tx.try_send(Err(ActyxOSCode::ERR_NODE_UNREACHABLE.with_message("not connected")))
            .log();
        return false;
    };
    if choices.iter().all(|c| !protos.contains(*c)) {
        tx.try_send(Err(ActyxOSCode::ERR_UNSUPPORTED.with_message(format!(
            "remote node supports none of {:?}, it supports {:?}",
            choices, protos
        ))))
        .log();
        true
    } else {
        false
    }
}

pub async fn request<T, F, U, F2>(task: &mut Sender<Task>, f: F, extract: F2) -> ActyxOSResult<Vec<U>>
where
    F: FnOnce(Sender<ActyxOSResult<T>>) -> Task + Send + 'static,
    T: Send + 'static,
    F2: Fn(ActyxOSResult<T>) -> ActyxOSResult<U> + Send + 'static,
    U: Send + 'static,
{
    let (tx, mut rx) = channel(10);
    task.feed(f(tx)).await?;
    let mut v = Vec::new();
    while let Some(msg) = rx.next().await {
        v.push(extract(msg)?);
    }
    Ok(v)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventDiagnostic {
    Event(EventResponse<Payload>),
    AntiEvent(EventResponse<Payload>),
    Diagnostic(Diagnostic),
}

pub async fn request_events(
    task: &mut Sender<Task>,
    peer_id: PeerId,
    req: EventsRequest,
) -> ActyxOSResult<BoxStream<'static, ActyxOSResult<EventDiagnostic>>> {
    let (tx, rx) = channel(128);
    task.feed(Task::Events(peer_id, req, tx)).await?;
    Ok(rx
        .filter_map(|m| match m {
            Ok(EventsResponse::Event(ev)) => ready(Some(Ok(EventDiagnostic::Event(ev)))),
            Ok(EventsResponse::AntiEvent(ev)) => ready(Some(Ok(EventDiagnostic::AntiEvent(ev)))),
            Ok(EventsResponse::Error { message }) => {
                ready(Some(Err(ActyxOSCode::ERR_INVALID_INPUT.with_message(message))))
            }
            Ok(EventsResponse::Diagnostic(d)) => ready(Some(Ok(EventDiagnostic::Diagnostic(d)))),
            Ok(EventsResponse::OffsetMap { offsets }) => {
                tracing::info!("received OffsetMap covering {} events", offsets.size());
                ready(None)
            }
            Ok(x @ EventsResponse::Offsets(..) | x @ EventsResponse::Publish(..)) => ready(Some(Err(
                ActyxOSCode::ERR_INTERNAL_ERROR.with_message(format!("unexpected: {:?}", x)),
            ))),
            Ok(x @ EventsResponse::FutureCompat) => ready(Some(Err(
                ActyxOSCode::ERR_INTERNAL_ERROR.with_message(format!("{:?}", x))
            ))),
            Err(e) => ready(Some(Err(e))),
        })
        .boxed())
}

pub async fn request_single<T, F, U, F2>(task: &mut Sender<Task>, f: F, extract: F2) -> ActyxOSResult<U>
where
    F: FnOnce(Sender<ActyxOSResult<T>>) -> Task + Send + 'static,
    T: Send + 'static,
    F2: Fn(T) -> ActyxOSResult<U> + Send + 'static,
    U: std::fmt::Debug + Send + 'static,
{
    #[allow(clippy::redundant_closure)]
    let v = request(task, f, move |r| r.and_then(|r| extract(r))).await?;
    if v.len() != 1 {
        return Err(ActyxOSCode::ERR_IO.with_message(format!("expected 1 result, got {:?}", v)));
    }
    Ok(v.into_iter().next().unwrap())
}

pub async fn request_banyan(task: &mut Sender<Task>, peer_id: PeerId, req: BanyanRequest) -> ActyxOSResult<()> {
    let (tx, mut rx) = channel(1);
    task.feed(Task::Banyan(peer_id, req, tx)).await?;
    let resp = rx.next().await;
    resp.ok_or_else(|| ActyxOSCode::ERR_INTERNAL_ERROR.with_message("stream ended abruptly"))?
        .and_then(|banyan| match banyan {
            BanyanResponse::Ok => Ok(()),
            BanyanResponse::Error(e) => Err(ActyxOSError::new(
                ActyxOSCode::ERR_IO,
                format!("error from Actyx node: {}", e),
            )),
            BanyanResponse::Future => Err(ActyxOSError::new(
                ActyxOSCode::ERR_IO,
                "message from Actyx node from the future",
            )),
        })
}

pub async fn connect(task: &mut Sender<Task>, auth: Authority) -> ActyxOSResult<PeerId> {
    let v = request(task, |tx| Task::Connect(auth, tx), Ok).await?;
    let mut err = None;
    for res in v {
        if res.is_ok() {
            return res;
        } else if err.is_none() {
            err = Some(res);
        }
    }
    err.unwrap_or_else(|| Err(ActyxOSCode::ERR_INTERNAL_ERROR.with_message("no connection results returned")))
}

trait SendErr {
    fn log(self);
}
impl<T: Debug> SendErr for Result<(), TrySendError<ActyxOSResult<T>>> {
    fn log(self) {
        if let Err(e) = self {
            let error = if e.is_disconnected() {
                "channel disconnected"
            } else {
                "channel full"
            };
            tracing::warn!("cannot send error {}: {}", e.into_inner().unwrap_err(), error);
        }
    }
}
