use crate::private_key::AxPrivateKey;
use actyx_sdk::NodeId;
use crypto::PublicKey;
use derive_more::From;
use futures::{
    future::{ready, Either},
    stream, FutureExt, Stream, StreamExt,
};
use libp2p::{
    core::{multiaddr::Protocol, muxing::StreamMuxerBox, transport::Boxed, ConnectedPoint, Multiaddr, PeerId},
    identify::{Identify, IdentifyConfig, IdentifyEvent},
    identity,
    ping::{Ping, PingConfig, PingEvent, PingSuccess},
    request_response::{
        ProtocolSupport, RequestResponse, RequestResponseConfig, RequestResponseEvent, RequestResponseMessage,
    },
    swarm::{dial_opts::DialOpts, Swarm, SwarmBuilder, SwarmEvent},
    NetworkBehaviour,
};
use libp2p_streaming_response::v2::StreamingResponse;
use std::{collections::BTreeSet, convert::TryFrom, fmt, num::NonZeroU16, str::FromStr, time::Duration};
use tracing::*;
use util::formats::{
    admin_protocol::{AdminRequest, AdminResponse},
    ax_err,
    banyan_protocol::{BanyanProtocol, BanyanProtocolName, BanyanRequest, BanyanResponse},
    events_protocol::{EventsProtocol, EventsRequest, EventsResponse},
    ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt, AdminProtocol,
};
use util::SocketAddrHelper;

#[derive(Debug, Clone)]
pub struct NodeConnection {
    pub original: String,
    pub host: SocketAddrHelper,
    peer_id: Option<PeerId>,
}

impl Default for NodeConnection {
    fn default() -> Self {
        "localhost:4458".parse().unwrap()
    }
}

impl FromStr for NodeConnection {
    type Err = ActyxOSError;
    fn from_str(s: &str) -> ActyxOSResult<Self> {
        // try to extract peer id if it's a valid multiaddr
        if let Ok(mut m) = Multiaddr::from_str(s) {
            let p = strip_peer_id(&mut m);
            let addr = SocketAddrHelper::try_from(m).expect("Valid multiaddr");
            NodeConnection::new(s.to_owned(), addr, p)
        } else {
            let addr = SocketAddrHelper::from_host(s, NonZeroU16::new(4458).unwrap()).ax_invalid_input()?;
            NodeConnection::new(s.to_owned(), addr, None)
        }
    }
}

impl NodeConnection {
    pub fn new(original: String, host: SocketAddrHelper, peer_id: Option<PeerId>) -> ActyxOSResult<Self> {
        Ok(Self {
            original,
            host,
            peer_id,
        })
    }

    /// Tries to establish a connection to the remote ActyxOS node, and returns
    /// a connection handle upon success.
    pub async fn connect(&self, key: &AxPrivateKey) -> ActyxOSResult<Connected> {
        let kp = key.to_libp2p_pair();
        let public_key = kp.public();
        let (peer_id, transport) = mk_transport(kp).await?;

        let mut request_response_config = RequestResponseConfig::default();
        request_response_config.set_request_timeout(Duration::from_secs(120));
        let protocol = RequestBehaviour {
            admin_api: StreamingResponse::new(Default::default()),
            events_api: StreamingResponse::new(Default::default()),
            banyan_api: RequestResponse::new(
                BanyanProtocol::default(),
                [(BanyanProtocolName, ProtocolSupport::Outbound)],
                request_response_config,
            ),
            ping: Ping::new(PingConfig::new().with_keep_alive(true)),
            identify: Identify::new(
                IdentifyConfig::new("Actyx".to_owned(), public_key).with_initial_delay(Duration::from_secs(0)),
            ),
        };
        let mut swarm = SwarmBuilder::new(transport, protocol, peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

        let (remote_peer_id, _connection) = poll_until_connected(&mut swarm, self.host.clone().to_multiaddrs()).await?;
        if let Some(expected) = self.peer_id {
            if expected != remote_peer_id {
                return Err(ActyxOSError::new(
                    ActyxOSCode::ERR_NODE_AUTH,
                    "remote PeerId does not match expectation!",
                ));
            }
        }
        let protocols = Self::await_identify(&mut swarm).await.into_iter().collect();

        Ok(Connected {
            remote_peer_id,
            swarm,
            protocols,
        })
    }

    async fn await_identify(swarm: &mut Swarm<RequestBehaviour>) -> Vec<String> {
        loop {
            let message = swarm.next().await.expect("swarm exited");
            tracing::debug!("waiting for identify: {:?}", message);
            match message {
                SwarmEvent::Behaviour(OutEvent::Identify(IdentifyEvent::Error { .. })) => {
                    // Actyx v2.0.x didn’t have the identify protocol
                    return vec!["/actyx/admin/1.0.0".to_owned()];
                }
                SwarmEvent::Behaviour(OutEvent::Identify(IdentifyEvent::Received { info, .. })) => {
                    return info.protocols
                }
                _ => {}
            }
        }
    }
}

pub struct Connected {
    remote_peer_id: PeerId,
    swarm: Swarm<RequestBehaviour>,
    protocols: BTreeSet<String>,
}

impl From<&Connected> for NodeInfo {
    fn from(this: &Connected) -> NodeInfo {
        NodeInfo {
            id: to_node_id(this.remote_peer_id),
            peer_id: this.remote_peer_id,
        }
    }
}

impl fmt::Debug for Connected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Connected to {}", self.remote_peer_id)
    }
}

impl Connected {
    fn send(&mut self, request: Either<AdminRequest, EventsRequest>) {
        let swarm = &mut self.swarm;
        let remote = self.remote_peer_id;
        match request {
            Either::Left(request) => swarm.behaviour_mut().admin_api.request(remote, request),
            Either::Right(request) => swarm.behaviour_mut().events_api.request(remote, request),
        };
    }

    pub async fn shutdown(&mut self) -> ActyxOSResult<()> {
        self.send(Either::Left(AdminRequest::NodesShutdown));
        match self.wait_for_next_response().await {
            Err(e) if e.code() == ActyxOSCode::ERR_NODE_UNREACHABLE => Ok(()),
            x => Err(ActyxOSError::new(
                ActyxOSCode::ERR_INTERNAL_ERROR,
                format!("unexpected response for shutdown: {:?}", x),
            )),
        }
    }

    pub async fn request(&mut self, request: AdminRequest) -> ActyxOSResult<AdminResponse> {
        self.send(Either::Left(request));

        // It can be assumed that there's an established connection to the
        // remote peer, so a conservative timeout is fine to use.
        match tokio::time::timeout(Duration::from_secs(5), self.wait_for_next_response()).await {
            Ok(resp) => resp,
            Err(_) => ax_err(
                ActyxOSCode::ERR_NODE_UNREACHABLE,
                format!("Timeout while waiting for answer from {}.", self.remote_peer_id),
            ),
        }
    }

    async fn wait_for_next_response(&mut self) -> ActyxOSResult<AdminResponse> {
        while let Some(message) = self.swarm.next().await {
            match message {
                SwarmEvent::Behaviour(OutEvent::Admin(StreamingResponseEvent::ResponseReceived {
                    payload, ..
                })) => return payload,

                SwarmEvent::ConnectionClosed { peer_id, .. } if peer_id == self.remote_peer_id => {
                    return ax_err(
                        ActyxOSCode::ERR_NODE_UNREACHABLE,
                        format!("Connection to {} unexpectedly closed.", self.remote_peer_id),
                    );
                }
                SwarmEvent::Behaviour(OutEvent::Ping(PingEvent {
                    peer,
                    result: Ok(success),
                })) => {
                    if let PingSuccess::Ping { rtt } = success {
                        info!("RTT to {}: {:?}", peer, rtt);
                    }
                }
                m => {
                    debug!("Unknown event {:?}", m);
                }
            }
        }
        error!("Swarm exited unexpectedly");
        ax_err(ActyxOSCode::ERR_INTERNAL_ERROR, "Swarm exited unexpectedly".into())
    }

    pub async fn request_events(
        &mut self,
        request: EventsRequest,
    ) -> ActyxOSResult<impl Stream<Item = EventsResponse> + Unpin + Send + '_> {
        if !self.protocols.contains("/actyx/events/v2") {
            return Err(ActyxOSError::new(
                ActyxOSCode::ERR_UNSUPPORTED,
                "Events API tunneling not supported by Actyx node, please update to a newer version of Actyx",
            ));
        }
        self.send(Either::Right(request));
        Ok(stream::unfold(&mut self.swarm, |s| {
            async move {
                let ev = s.next().await.expect("swarm exited");
                tracing::debug!("got swarm event {:?}", ev);
                match ev {
                    SwarmEvent::Behaviour(OutEvent::Events(e)) => match e {
                        StreamingResponseEvent::ResponseReceived { payload, .. } => Some((vec![payload], s)),
                        _ => None,
                    },
                    SwarmEvent::ConnectionClosed { .. } => None,
                    _ => Some((vec![], s)),
                }
            }
            .boxed()
        })
        .filter_map(|mut v| ready(v.pop())))
    }

    pub async fn request_banyan(&mut self, request: BanyanRequest) -> ActyxOSResult<BanyanResponse> {
        if !self.protocols.contains("/actyx/banyan/create") {
            return Err(ActyxOSError::new(
                ActyxOSCode::ERR_UNSUPPORTED,
                "Dump upload functionality is only available in Actyx v2.9+",
            ));
        }
        let id = self
            .swarm
            .behaviour_mut()
            .banyan_api
            .send_request(&self.remote_peer_id, request);
        while let Some(message) = self.swarm.next().await {
            match message {
                SwarmEvent::Behaviour(OutEvent::Banyan(RequestResponseEvent::Message {
                    peer,
                    message: RequestResponseMessage::Response { request_id, response },
                })) if peer == self.remote_peer_id && request_id == id => return Ok(response),

                SwarmEvent::ConnectionClosed { peer_id, .. } if peer_id == self.remote_peer_id => {
                    return ax_err(
                        ActyxOSCode::ERR_NODE_UNREACHABLE,
                        format!("Connection to {} unexpectedly closed.", self.remote_peer_id),
                    );
                }
                SwarmEvent::Behaviour(OutEvent::Ping(PingEvent {
                    peer,
                    result: Ok(success),
                })) => {
                    if let PingSuccess::Ping { rtt } = success {
                        info!("RTT to {}: {:?}", peer, rtt);
                    }
                }
                m => {
                    debug!("Unknown event {:?}", m);
                }
            }
        }
        error!("Swarm exited unexpectedly");
        ax_err(ActyxOSCode::ERR_INTERNAL_ERROR, "Swarm exited unexpectedly".into())
    }
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: NodeId,
    pub peer_id: PeerId,
}

/// Converts a libp2p PeerId to a NodeId.
/// Panics if the PeerId didn't originate from an Actyx node.
pub fn to_node_id(peer_id: PeerId) -> NodeId {
    let pk = PublicKey::try_from(peer_id).expect("Not an ActyxOS Node on the other side");
    pk.into()
}
/// for a multiaddr that ends with a peer id, this strips this suffix.
/// Rust-libp2p only supports dialing to an address without providing the peer id.
pub fn strip_peer_id(addr: &mut Multiaddr) -> Option<PeerId> {
    let last = addr.pop();
    match last {
        Some(Protocol::P2p(peer_id)) => PeerId::from_multihash(peer_id).ok(),
        Some(other) => {
            addr.push(other);
            None
        }
        _ => None,
    }
}

#[derive(Debug, From)]
pub enum OutEvent {
    Admin(StreamingResponseEvent<AdminProtocol>),
    Events(StreamingResponseEvent<EventsProtocol>),
    Banyan(RequestResponseEvent<BanyanRequest, BanyanResponse>),
    Ping(PingEvent),
    Identify(IdentifyEvent),
}

#[derive(NetworkBehaviour)]
#[behaviour(event_process = false, out_event = "OutEvent")]
pub struct RequestBehaviour {
    admin_api: StreamingResponse<AdminProtocol>,
    events_api: StreamingResponse<EventsProtocol>,
    banyan_api: RequestResponse<BanyanProtocol>,
    ping: Ping,
    identify: Identify,
}

async fn mk_transport(keypair: identity::Keypair) -> ActyxOSResult<(PeerId, Boxed<(PeerId, StreamMuxerBox)>)> {
    let peer_id = keypair.public().to_peer_id();
    let transport = swarm::transport::build_transport(keypair, None, Duration::from_secs(20))
        .await
        .ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "Error creating libp2p transport")?;
    Ok((peer_id, transport))
}

/// Dials all provided `potential_addresses`, and yields with the first
/// successful established one.
async fn poll_until_connected(
    mut swarm: &mut Swarm<RequestBehaviour>,
    potential_addresses: impl Iterator<Item = Multiaddr>,
) -> ActyxOSResult<(PeerId, Multiaddr)> {
    let mut to_try = 0usize;
    for addr in potential_addresses {
        info!("Trying to connect to {}", addr);
        Swarm::dial(&mut swarm, DialOpts::unknown_peer_id().address(addr).build()).expect("Connection limit exceeded");
        to_try += 1;
    }
    while let Some(event) = swarm.next().await {
        match event {
            SwarmEvent::ConnectionEstablished { endpoint, peer_id, .. } => {
                let addr = match endpoint {
                    ConnectedPoint::Dialer { address } => address,
                    ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
                };

                info!("connected to {} ({})", peer_id, addr);
                return Ok((peer_id, addr));
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                debug!("Listening on {}", address);
            }
            SwarmEvent::OutgoingConnectionError { peer_id: _, error } => {
                to_try -= 1;
                if to_try == 0 {
                    return ax_err(ActyxOSCode::ERR_NODE_UNREACHABLE, error.to_string());
                } else {
                    info!("{}, still got {} other connections to try", error, to_try);
                }
            }
            m => {
                warn!("Uexpected message {:?}", m);
            }
        }
    }
    error!("Swarm exited unexpectedly");
    ax_err(ActyxOSCode::ERR_INTERNAL_ERROR, "Swarm exited unexpectedly".into())
}
