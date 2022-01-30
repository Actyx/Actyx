use crate::{cmd::Authority, private_key::AxPrivateKey};
use actyx_sdk::{service::EventResponse, NodeId, Payload};
use anyhow::anyhow;
use crypto::PublicKey;
use derive_more::From;
use futures::{
    channel::mpsc::{channel, Sender},
    future::{poll_fn, ready},
    stream::BoxStream,
    Future, SinkExt, StreamExt,
};
use libp2p::{
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    ping::{Ping, PingConfig, PingEvent},
    request_response::{
        ProtocolSupport, RequestId, RequestResponse, RequestResponseConfig, RequestResponseEvent,
        RequestResponseMessage,
    },
    swarm::{SwarmBuilder, SwarmEvent},
    NetworkBehaviour, Swarm,
};
use libp2p_streaming_response::v2::{RequestReceived, Response, StreamingResponse, StreamingResponseConfig};
use std::{collections::HashMap, convert::TryFrom, task::Poll, time::Duration};
use swarm::transport::build_transport;
use util::formats::{
    banyan_protocol::{BanyanProtocol, BanyanProtocolName, BanyanRequest, BanyanResponse},
    events_protocol::{EventsProtocol, EventsRequest, EventsResponse},
    ActyxOSCode, ActyxOSError, ActyxOSResult, AdminProtocol, AdminRequest, AdminResponse,
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
    Admin(AdminRequest, Sender<Response<ActyxOSResult<AdminResponse>>>),
    Events(EventsRequest, Sender<Response<EventsResponse>>),
    Banyan(BanyanRequest, Sender<BanyanResponse>),
    NodeId(Sender<ActyxOSResult<NodeId>>),
}

pub fn connect(
    key: AxPrivateKey,
    authority: Authority,
) -> (impl Future<Output = anyhow::Result<()>> + Send + 'static, Sender<Task>) {
    let (tx, mut rx) = channel(10);

    let task = async move {
        let key_pair = key.to_libp2p_pair();
        let public_key = key_pair.public();
        let local_peer_id = public_key.to_peer_id();
        let transport = build_transport(key_pair, None, Duration::from_secs(20)).await?;
        let behaviour = Behaviour {
            admin: StreamingResponse::new(
                StreamingResponseConfig::default().with_request_timeout(Duration::from_secs(20)),
            ),
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
            swarm.dial(addr.clone())?;
            tries += 1;
        }

        let mut peer_id = None;
        let info = loop {
            match swarm.next().await.ok_or(anyhow!("swarm stopped"))? {
                SwarmEvent::Behaviour(OutEvent::Identify(IdentifyEvent::Received { info, peer_id: seen }))
                    if Some(seen) == peer_id =>
                {
                    break Some(info)
                }
                SwarmEvent::Behaviour(OutEvent::Identify(IdentifyEvent::Error { error, peer_id: seen }))
                    if Some(seen) == peer_id =>
                {
                    // Actyx v2.0.x didn’t have the identify protocol
                    break None;
                }
                SwarmEvent::Behaviour(OutEvent::Identify(_)) => {
                    tracing::info!("ignoring secondary connection");
                }
                SwarmEvent::ConnectionEstablished {
                    peer_id: seen,
                    endpoint,
                    ..
                } => {
                    tracing::info!(peer = %seen, addr = %endpoint.get_remote_address(), "connected to {}", authority.original);
                    peer_id = Some(seen);
                }
                SwarmEvent::OutgoingConnectionError { peer_id, error } => {
                    tries -= 1;
                    tracing::info!("dial error: {}", error);
                    if tries == 0 {
                        return Err(error.into());
                    }
                }
                _ => {}
            }
        };
        let peer_id = peer_id.unwrap();

        let peer_public_key = info.as_ref().and_then(|e| PublicKey::try_from(&e.public_key).ok());
        let peer_protos = info
            .as_ref()
            .map(|i| i.protocols.clone())
            .unwrap_or_else(|| vec!["/actyx/admin/1.0.0".to_owned()]);

        poll_fn(move |cx| {
            let mut banyan_channels = HashMap::<RequestId, Sender<BanyanResponse>>::new();
            loop {
                match swarm.poll_next_unpin(cx) {
                    Poll::Ready(Some(ev)) => {
                        tracing::info!("swarm event: {:?}", ev);
                        if let SwarmEvent::Behaviour(OutEvent::Banyan(RequestResponseEvent::Message {
                            message: RequestResponseMessage::Response { request_id, response },
                            ..
                        })) = ev
                        {
                            if let Some(mut channel) = banyan_channels.remove(&request_id) {
                                if channel.try_send(response).is_err() {
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
                match rx.poll_next_unpin(cx) {
                    Poll::Ready(Some(task)) => match task {
                        Task::Admin(request, channel) => swarm.behaviour_mut().admin.request(peer_id, request, channel),
                        Task::Events(request, channel) => {
                            swarm.behaviour_mut().events.request(peer_id, request, channel);
                        }
                        Task::Banyan(request, channel) => {
                            let id = swarm.behaviour_mut().banyan.send_request(&peer_id, request);
                            banyan_channels.insert(id, channel);
                        }
                        Task::NodeId(mut channel) => {
                            let node_id = peer_public_key.map(NodeId::from).ok_or(ActyxOSError::new(
                                ActyxOSCode::ERR_UNSUPPORTED,
                                "remote node does not provide a public key",
                            ));
                            if channel.try_send(node_id).is_err() {
                                tracing::warn!("dropping node ID response");
                            }
                        }
                    },
                    Poll::Ready(None) => return Poll::Ready(Ok(())),
                    Poll::Pending => break,
                }
            }
            Poll::Pending
        })
        .await
    };

    (task, tx)
}

pub async fn request<T, F, U, F2>(task: &mut Sender<Task>, f: F, extract: F2) -> ActyxOSResult<Vec<U>>
where
    F: FnOnce(Sender<Response<T>>) -> Task + Send + 'static,
    T: Send + 'static,
    F2: Fn(T) -> ActyxOSResult<U> + Send + 'static,
    U: Send + 'static,
{
    let (tx, mut rx) = channel(10);
    task.feed(f(tx)).await?;
    let mut v = Vec::new();
    loop {
        match rx.next().await {
            Some(Response::Msg(m)) => {
                v.push(extract(m)?);
            }
            Some(Response::Finished) => break,
            Some(Response::Error(e)) => return Err(ActyxOSCode::ERR_IO.with_message(e.to_string())),
            None => return Err(ActyxOSCode::ERR_IO.with_message("stream ended abruptly")),
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
        .take_while(|m| ready(!matches!(m, Response::Finished)))
        .filter_map(|m| match m {
            Response::Msg(EventsResponse::Event(ev)) => ready(Some(Ok(ev))),
            Response::Msg(EventsResponse::Error { message }) => {
                ready(Some(Err(ActyxOSCode::ERR_INVALID_INPUT.with_message(message))))
            }
            Response::Msg(EventsResponse::Diagnostic(d)) => {
                tracing::warn!("AQL diagnostic: {}", d);
                ready(None)
            }
            Response::Msg(x) => ready(Some(Err(
                ActyxOSCode::ERR_INTERNAL_ERROR.with_message(format!("{:?}", x))
            ))),
            Response::Error(e) => ready(Some(Err(ActyxOSCode::ERR_IO.with_message(e.to_string())))),
            Response::Finished => unreachable!(),
        })
        .boxed())
}

pub async fn request_single<T, F, U, F2>(task: &mut Sender<Task>, f: F, extract: F2) -> ActyxOSResult<U>
where
    F: FnOnce(Sender<Response<T>>) -> Task + Send + 'static,
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
    let s = resp.ok_or_else(|| ActyxOSCode::ERR_INTERNAL_ERROR.with_message("stream ended abruptly"))?;
    match s {
        BanyanResponse::Ok => Ok(()),
        BanyanResponse::Error(e) => Err(ActyxOSError::new(
            ActyxOSCode::ERR_IO,
            format!("error from Actyx node: {}", e),
        )),
        BanyanResponse::Future => Err(ActyxOSError::new(
            ActyxOSCode::ERR_IO,
            "message from Actyx node from the future",
        )),
    }
}
