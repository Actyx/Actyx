use crate::{cmd::Authority, private_key::AxPrivateKey};
use actyx_sdk::{service::EventResponse, NodeId, Payload};
use anyhow::anyhow;
use crypto::PublicKey;
use derive_more::From;
use futures::{
    channel::mpsc::{self, channel, Receiver, Sender},
    future::{poll_fn, ready},
    stream::BoxStream,
    Future, FutureExt, SinkExt, StreamExt,
};
use libp2p::{
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    ping::{Ping, PingConfig, PingEvent},
    request_response::{
        ProtocolSupport, RequestId, RequestResponse, RequestResponseConfig, RequestResponseEvent,
        RequestResponseMessage,
    },
    swarm::{SwarmBuilder, SwarmEvent},
    NetworkBehaviour,
};
use libp2p_streaming_response::v2::{RequestReceived, Response, StreamingResponse, StreamingResponseConfig};
use std::{collections::HashMap, convert::TryFrom, task::Poll, time::Duration};
use swarm::transport::build_transport;
use util::formats::{
    banyan_protocol::{BanyanProtocol, BanyanProtocolName, BanyanRequest, BanyanResponse},
    events_protocol::{EventsProtocol, EventsRequest, EventsResponse},
    ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt, AdminProtocol, AdminRequest, AdminResponse,
};

#[derive(NetworkBehaviour)]
#[behaviour(event_process = false, out_event = "OutEvent")]
struct Behaviour {
    admin: StreamingResponse<AdminProtocol>,
    events: StreamingResponse<EventsProtocol>,
    banyan: RequestResponse<BanyanProtocol>,
    ping: Ping,
    identify: Identify,
}

#[derive(Debug, From)]
enum OutEvent {
    Admin(RequestReceived<AdminProtocol>),
    Events(RequestReceived<EventsProtocol>),
    Banyan(RequestResponseEvent<BanyanRequest, BanyanResponse>),
    Ping(PingEvent),
    Identify(IdentifyEvent),
}

pub enum Task {
    Admin(AdminRequest, Sender<ActyxOSResult<AdminResponse>>),
    Events(EventsRequest, Sender<ActyxOSResult<EventsResponse>>),
    Banyan(BanyanRequest, Sender<ActyxOSResult<BanyanResponse>>),
    NodeId(Sender<ActyxOSResult<NodeId>>),
}

impl std::fmt::Debug for Task {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admin(arg0, _arg1) => f.debug_tuple("Admin").field(arg0).finish(),
            Self::Events(arg0, _arg1) => f.debug_tuple("Events").field(arg0).finish(),
            Self::Banyan(arg0, _arg1) => f.debug_tuple("Banyan").field(arg0).finish(),
            Self::NodeId(_arg0) => f.debug_tuple("NodeId").finish(),
        }
    }
}

pub async fn connect(
    key: AxPrivateKey,
    authority: Authority,
) -> ActyxOSResult<(impl Future<Output = ()> + Send + 'static, Sender<Task>)> {
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
            StreamingResponseConfig::default().with_request_timeout(Duration::from_secs(20)),
        ),
        banyan: RequestResponse::new(
            BanyanProtocol::default(),
            [(BanyanProtocolName, ProtocolSupport::Outbound)],
            RequestResponseConfig::default(),
        ),
        ping: Ping::new(PingConfig::new().with_keep_alive(true)),
        identify: Identify::new(IdentifyConfig::new("Actyx".to_owned(), public_key)),
    };
    let mut swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
        .executor(Box::new(|task| {
            tokio::spawn(task);
        }))
        .build();
    let mut tries = 0;
    for addr in &authority.addrs {
        swarm
            .dial(addr.clone())
            .ax_err_ctx(ActyxOSCode::ERR_IO, "cannot dial node")?;
        tries += 1;
    }

    let mut peer_id = None;
    let info = loop {
        match swarm
            .next()
            .await
            .ok_or_else(|| ActyxOSCode::ERR_INTERNAL_ERROR.with_message("swarm stopped"))?
        {
            SwarmEvent::Behaviour(OutEvent::Identify(IdentifyEvent::Received { info, peer_id: seen })) => {
                tracing::info!(peer = %seen, "got identify info");
                break Some(info);
            }
            SwarmEvent::Behaviour(OutEvent::Identify(IdentifyEvent::Error { error, peer_id: seen })) => {
                // Actyx v2.0.x didn’t have the identify protocol
                tracing::info!(peer = %seen, error = ?error, "got identify error");
                break None;
            }
            SwarmEvent::ConnectionEstablished {
                peer_id: seen,
                endpoint,
                ..
            } => {
                tracing::info!(peer = %seen, addr = %endpoint.get_remote_address(), "connected to {}", authority.original);
                peer_id = Some(seen);
            }
            SwarmEvent::OutgoingConnectionError { error, .. } => {
                tries -= 1;
                if tries == 0 {
                    return Err(ActyxOSCode::ERR_IO.with_message(error.to_string()));
                }
            }
            x => {
                tracing::info!("swarm event: {:?}", x);
            }
        }
    };
    let peer_id = peer_id.unwrap();

    let peer_public_key = info.as_ref().and_then(|e| PublicKey::try_from(&e.public_key).ok());
    let peer_protos = info
        .as_ref()
        .map(|i| i.protocols.clone())
        .unwrap_or_else(|| vec!["/actyx/admin/1.0.0".to_owned()]);

    let task = poll_fn(move |cx| {
        let mut banyan_channels = HashMap::<RequestId, Sender<ActyxOSResult<BanyanResponse>>>::new();
        loop {
            tracing::debug!("polling swarm");
            match swarm.poll_next_unpin(cx) {
                Poll::Ready(Some(ev)) => {
                    tracing::info!("swarm event: {:?}", ev);
                    if let SwarmEvent::Behaviour(OutEvent::Banyan(RequestResponseEvent::Message {
                        message: RequestResponseMessage::Response { request_id, response },
                        ..
                    })) = ev
                    {
                        if let Some(mut channel) = banyan_channels.remove(&request_id) {
                            if channel.try_send(Ok(response)).is_err() {
                                tracing::warn!("dropping banyan response");
                            }
                        } else {
                            tracing::warn!("got response for unknown ID {}", request_id);
                        }
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
                        Task::Admin(request, mut channel) => {
                            if unsupported_proto(
                                &*peer_protos,
                                &["/actyx/admin/1.0.0", "/actyx/admin/1.1"],
                                &mut channel,
                            ) {
                                continue;
                            }
                            let (tx, rx) = mpsc::channel(128);
                            swarm.behaviour_mut().admin.request(peer_id, request, tx);
                            forward_stream(rx, channel, |ev| ev);
                        }
                        Task::Events(request, mut channel) => {
                            if unsupported_proto(&*peer_protos, &["/actyx/events/v2", "/actyx/events/v3"], &mut channel)
                            {
                                continue;
                            }
                            let (tx, rx) = mpsc::channel(128);
                            swarm.behaviour_mut().events.request(peer_id, request, tx);
                            forward_stream(rx, channel, Ok);
                        }
                        Task::Banyan(request, mut channel) => {
                            if unsupported_proto(&*peer_protos, &["/actyx/banyan/create"], &mut channel) {
                                continue;
                            }
                            let id = swarm.behaviour_mut().banyan.send_request(&peer_id, request);
                            banyan_channels.insert(id, channel);
                        }
                        Task::NodeId(mut channel) => {
                            let node_id = peer_public_key.map(NodeId::from).ok_or_else(|| {
                                ActyxOSError::new(
                                    ActyxOSCode::ERR_UNSUPPORTED,
                                    "remote node does not provide a public key",
                                )
                            });
                            if channel.try_send(node_id).is_err() {
                                tracing::warn!("dropping node ID response");
                            }
                        }
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
        if let Err(e) = res {
            tracing::error!("event loop error: {}", e);
        }
    });

    Ok((task, tx))
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

fn unsupported_proto<T>(protos: &[String], choices: &[&str], tx: &mut Sender<ActyxOSResult<T>>) -> bool {
    if protos.iter().all(|p| choices.iter().all(|c| c != p)) {
        if let Err(e) = tx.try_send(Err(ActyxOSCode::ERR_UNSUPPORTED.with_message(format!(
            "remote node supports none of {:?}, it supports {:?}",
            choices, protos
        )))) {
            tracing::warn!("cannot sent error: {}", e);
        }
        true
    } else {
        false
    }
}

pub async fn request<T, F, U, F2>(task: &mut Sender<Task>, f: F, extract: F2) -> ActyxOSResult<Vec<U>>
where
    F: FnOnce(Sender<ActyxOSResult<T>>) -> Task + Send + 'static,
    T: Send + 'static,
    F2: Fn(T) -> ActyxOSResult<U> + Send + 'static,
    U: Send + 'static,
{
    let (tx, mut rx) = channel(10);
    task.feed(f(tx)).await?;
    let mut v = Vec::new();
    while let Some(msg) = rx.next().await {
        match msg {
            Ok(m) => {
                v.push(extract(m)?);
            }
            Err(e) => return Err(e),
        }
    }
    Ok(v)
}

pub async fn request_events(
    task: &mut Sender<Task>,
    req: EventsRequest,
) -> ActyxOSResult<BoxStream<'static, ActyxOSResult<EventResponse<Payload>>>> {
    let (tx, rx) = channel(128);
    task.feed(Task::Events(req, tx)).await?;
    Ok(rx
        .filter_map(|m| match m {
            Ok(EventsResponse::Event(ev)) => ready(Some(Ok(ev))),
            Ok(EventsResponse::Error { message }) => {
                ready(Some(Err(ActyxOSCode::ERR_INVALID_INPUT.with_message(message))))
            }
            Ok(EventsResponse::Diagnostic(d)) => {
                tracing::warn!("AQL diagnostic: {}", d);
                ready(None)
            }
            Ok(x) => ready(Some(Err(
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
    let v = request(task, f, extract).await?;
    if v.len() != 1 {
        return Err(ActyxOSCode::ERR_IO.with_message(format!("expected 1 result, got {:?}", v)));
    }
    Ok(v.into_iter().next().unwrap())
}

pub async fn request_banyan(task: &mut Sender<Task>, req: BanyanRequest) -> ActyxOSResult<()> {
    let (tx, mut rx) = channel(1);
    task.feed(Task::Banyan(req, tx)).await?;
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
