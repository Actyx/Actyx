use crate::private_key::AxPrivateKey;
use actyxos_sdk::NodeId;
use crypto::PublicKey;
use derive_more::From;
use libp2p::{
    core::{multiaddr::Protocol, muxing::StreamMuxerBox, transport::Boxed, ConnectedPoint, Multiaddr, PeerId},
    identity,
    ping::{Ping, PingConfig, PingEvent, PingSuccess},
    swarm::{Swarm, SwarmBuilder, SwarmEvent},
    NetworkBehaviour,
};
use libp2p_streaming_response::StreamingResponse;
use std::{convert::TryFrom, fmt, str::FromStr, time::Duration};
use tracing::*;
use util::formats::{
    admin_protocol::{AdminRequest, AdminResponse},
    ax_err, ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt, AdminProtocol,
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
            let addr = SocketAddrHelper::from_host(s, 4458).ax_invalid_input()?;
            NodeConnection::new(s.to_owned(), addr, None)
        }
    }
}

struct Connected {
    remote_peer_id: PeerId,
    swarm: Swarm<RequestBehaviour>,
    connection: Multiaddr,
}

impl fmt::Debug for Connected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Connected to {}", self.remote_peer_id)
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
    async fn establish_connection(&self, keypair: identity::Keypair) -> ActyxOSResult<Connected> {
        let (peer_id, transport) = mk_transport(keypair).await?;

        let protocol = RequestBehaviour {
            admin_api: StreamingResponse::new(Default::default()),
            ping: Ping::new(PingConfig::new().with_keep_alive(true)),
        };
        let mut swarm = SwarmBuilder::new(transport, protocol, peer_id)
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

        let (remote_peer_id, connection) = poll_until_connected(&mut swarm, self.host.clone().to_multiaddrs()).await?;

        Ok(Connected {
            remote_peer_id,
            swarm,
            connection,
        })
    }

    pub async fn request(&mut self, key: &AxPrivateKey, request: AdminRequest) -> ActyxOSResult<AdminResponse> {
        let kp = key.to_libp2p_pair();
        let Connected {
            remote_peer_id,
            mut swarm,
            connection,
        } = self.establish_connection(kp).await?;
        swarm.behaviour_mut().admin_api.request(remote_peer_id, request);
        let node_info = NodeInfo {
            id: to_node_id(remote_peer_id),
            peer_id: remote_peer_id,
            connection,
        };

        // It can be assumed that there's an established connection to the
        // remote peer, so a conservative timeout is fine to use.
        match tokio::time::timeout(
            Duration::from_secs(5),
            Self::wait_for_next_response(&mut swarm, &node_info),
        )
        .await
        {
            Ok(resp) => resp,
            Err(_) => ax_err(
                ActyxOSCode::ERR_NODE_UNREACHABLE,
                format!("Timeout while waiting for answer from {}.", node_info.id),
            ),
        }
    }

    async fn wait_for_next_response(
        swarm: &mut Swarm<RequestBehaviour>,
        node_info: &NodeInfo,
    ) -> ActyxOSResult<AdminResponse> {
        loop {
            let message = swarm.next_event().await;
            match message {
                SwarmEvent::Behaviour(OutEvent::Admin(
                    libp2p_streaming_response::StreamingResponseEvent::ResponseReceived { payload, .. },
                )) => return payload,

                SwarmEvent::ConnectionClosed { peer_id, .. } if peer_id == node_info.peer_id => {
                    return ax_err(
                        ActyxOSCode::ERR_NODE_UNREACHABLE,
                        format!("Connection to {} unexpectedly closed.", node_info.id),
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
    }
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: NodeId,
    pub peer_id: PeerId,
    pub connection: Multiaddr,
}

/// Converts a libp2p PeerId to a NodeId.
/// Panics if the PeerId didn't originate from an Actyx node.
fn to_node_id(peer_id: PeerId) -> NodeId {
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
    Admin(libp2p_streaming_response::StreamingResponseEvent<AdminProtocol>),
    Ping(PingEvent),
}

#[derive(NetworkBehaviour)]
#[behaviour(event_process = false, out_event = "OutEvent")]
pub struct RequestBehaviour {
    admin_api: StreamingResponse<AdminProtocol>,
    ping: Ping,
}

async fn mk_transport(keypair: identity::Keypair) -> ActyxOSResult<(PeerId, Boxed<(PeerId, StreamMuxerBox)>)> {
    let peer_id = keypair.public().into_peer_id();
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
        Swarm::dial_addr(&mut swarm, addr).expect("Connection limit exceeded");
        to_try += 1;
    }
    loop {
        match swarm.next_event().await {
            SwarmEvent::ConnectionEstablished { endpoint, peer_id, .. } => {
                let addr = match endpoint {
                    ConnectedPoint::Dialer { address } => address,
                    ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
                };

                info!("connected to {} ({})", peer_id, addr);
                break Ok((peer_id, addr));
            }
            SwarmEvent::NewListenAddr(x) => {
                debug!("Listening on {}", x);
            }
            SwarmEvent::UnknownPeerUnreachableAddr { address, .. } | SwarmEvent::UnreachableAddr { address, .. } => {
                to_try -= 1;
                if to_try == 0 {
                    break ax_err(ActyxOSCode::ERR_NODE_UNREACHABLE, format!("{} is unreachable", address));
                } else {
                    info!(
                        "{} is unreachable, still got {} other connections to try",
                        address, to_try
                    );
                }
            }
            m => {
                warn!("Uexpected message {:?}", m);
            }
        }
    }
}
