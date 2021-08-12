use crate::{
    admin_protocol::{AdminRequest, Node},
    errors::{ax_err, ActyxOSCode},
    events_protocol::EventDiagnostic,
};
use actyx_sdk::{
    language::Query,
    service::{OffsetsResponse, Order, QueryRequest},
};
use admin_protocol::{AdminProtocol, AdminResponse, ConnectedNodeDetails, NodesInspectResponse, NodesLsResponse};
use crypto::PrivateKey;
use derive_more::From;
use errors::{ActyxOSError, ActyxOSResult};
use events_protocol::{EventsProtocol, EventsRequest, EventsResponse, NodeManagerEventsRes};
use futures::{channel::mpsc, future, select, Future, StreamExt, TryFutureExt};
use futures_timer::Delay;
use js_sys::Promise;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed, upgrade::AuthenticationVersion, ConnectedPoint},
    identity, noise,
    ping::{Ping, PingConfig, PingEvent},
    swarm::SwarmEvent,
    wasm_ext::{ffi, ExtTransport},
    yamux::YamuxConfig,
    Multiaddr, NetworkBehaviour, PeerId, Swarm, Transport,
};
use libp2p_streaming_response::{StreamingResponse, StreamingResponseEvent};
use log::{error, info};
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::Mutex;
use serde::Serialize;
use std::{collections::BTreeMap, io, sync::Arc, time::Duration};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;

mod admin_protocol;
mod errors;
mod events_protocol;

#[wasm_bindgen(start)]
pub fn main() {
    let _ = console_log::init_with_level(log::Level::Info);
    ::console_error_panic_hook::set_once();
    info!("Setup panic hook");
}

#[wasm_bindgen]
pub fn create_private_key() -> String {
    let key = PrivateKey::generate();
    key.to_string()
}

#[derive(Debug)]
enum Either<A, B> {
    Left(A),
    Right(B),
}

// TODO: handle streams
// TODO: handle multiple nodes!
type Channel = Either<
    (AdminRequest, mpsc::Sender<ActyxOSResult<AdminResponse>>),
    (EventsRequest, mpsc::Sender<ActyxOSResult<EventsResponse>>),
>;
static SWARMS: Lazy<Mutex<BTreeMap<Multiaddr, Arc<Mutex<mpsc::Sender<Channel>>>>>> = Lazy::new(|| Default::default());

#[wasm_bindgen]
pub struct ActyxAdminApi {
    host: Multiaddr,
    tx: Arc<Mutex<mpsc::Sender<Channel>>>,
}

fn to_promise(
    fut: impl Future<Output = std::result::Result<impl Serialize, impl std::fmt::Display>> + 'static,
) -> Promise {
    future_to_promise(async move {
        fut.await
            .map(|e| JsValue::from_serde(&e).unwrap())
            .map_err(|e| js_sys::Error::new(&format!("Error: {:#}", e)).into())
    })
}

macro_rules! request {
    ($tx:expr, $($either:ident)::+, $req:expr, $($resp:ident)::+ ) => {
        {
            let (tx, mut rx) = mpsc::channel(1);
            $tx
              .lock()
              .clone()
              .start_send($($either)::+(($req, tx)))
              .unwrap();

            async move {
                if let Some(r) = rx.next().await {
                    match r {
                        Ok($($resp)::+ (x)) => Ok(x),
                        Err(e)  => Err(e),
                        _ => ax_err(ActyxOSCode::ERR_INTERNAL_ERROR, "Unexpected response".into())
                    }
                } else {
                    ax_err(ActyxOSCode::ERR_NODE_UNREACHABLE, "".into())
                }

            }
        }
    };
}

macro_rules! admin {
    ($tx:expr, $req:expr, $($resp:ident)::+ ) => {
        request!($tx, Either::Left, $req, $($resp)::+)
    };
}

macro_rules! events {
    ($tx:expr, $req:expr, $($resp:ident)::+ ) => {
        request!($tx, Either::Right, $req, $($resp)::+)
    };
}

impl Drop for ActyxAdminApi {
    fn drop(&mut self) {
        if Arc::strong_count(&self.tx) == 2 {
            SWARMS.lock().remove(&self.host);
        }
    }
}

#[wasm_bindgen]
impl ActyxAdminApi {
    #[wasm_bindgen(constructor)]
    #[allow(unused_must_use)]
    pub fn new(host: String, private_key: String) -> Self {
        let addr: Multiaddr = format!("/ip4/{}/tcp/4459/ws", host)
            .parse()
            .expect("Invalid host. Only ipv4 format supported");
        let tx = SWARMS
            .lock()
            .entry(addr.clone())
            .or_insert_with(|| {
                let (tx, rx) = mpsc::channel(64);

                // TODO: Move this to a webworker.
                // Right now, this basically just spawns the promise to wherever.
                future_to_promise(async move {
                    run(&*private_key, rx).await.unwrap();
                    Ok("XX".into())
                });
                Arc::new(Mutex::new(tx))
            })
            .clone();
        Self { host: addr, tx }
    }

    // Events API
    fn _offsets(&self) -> impl Future<Output = ActyxOSResult<OffsetsResponse>> + 'static {
        events!(self.tx, EventsRequest::Offsets, EventsResponse::Offsets)
    }
    // Unfortunately promises can't be typed, they always end up as `Promise<any>` in the ts
    // definition file. Synchronous function can be annotated with `#[wasm_bindgen(typescript_type = "..")]`
    pub fn offsets(&mut self) -> Promise {
        to_promise(self._offsets())
    }

    // TODO stream
    fn _query(&self, query: String) -> impl Future<Output = ActyxOSResult<NodeManagerEventsRes>> {
        let mut swarm_tx = self.tx.lock().clone();
        async move {
            let request = QueryRequest {
                lower_bound: None,
                upper_bound: None,
                query: query
                    .parse()
                    .map_err(|e| ActyxOSCode::ERR_INVALID_INPUT.with_message(format!("{}", e)))?,
                order: Order::Asc,
            };
            let (tx, rx) = mpsc::channel(256);
            swarm_tx
                .start_send(Either::Right((EventsRequest::Query(request), tx)))
                .unwrap();

            let out = rx
                .filter_map(|x| async move {
                    match x {
                        Ok(EventsResponse::Diagnostic(d)) => Some(EventDiagnostic::Diagnostic(d)),
                        Ok(EventsResponse::Event(e)) => Some(EventDiagnostic::Event(e)),
                        // TODO err
                        _ => None,
                    }
                })
                .collect::<Vec<_>>()
                .await;
            Ok(NodeManagerEventsRes { events: Some(out) })
        }
    }

    pub fn query(&mut self, query: String) -> Promise {
        to_promise(self._query(query))
    }

    // Admin API
    fn _get_settings(&self, scope: String) -> impl Future<Output = ActyxOSResult<serde_json::Value>> + 'static {
        admin!(
            self.tx,
            AdminRequest::SettingsGet {
                scope: scope.into(),
                no_defaults: false,
            },
            AdminResponse::SettingsGetResponse
        )
    }
    pub fn get_settings(&mut self, scope: String) -> Promise {
        let fut = self._get_settings(scope);
        to_promise(fut)
    }

    fn _set_settings(
        &self,
        scope: String,
        json: serde_json::Value,
    ) -> impl Future<Output = ActyxOSResult<serde_json::Value>> + 'static {
        admin!(
            self.tx,
            AdminRequest::SettingsSet {
                scope,
                json,
                ignore_errors: false,
            },
            AdminResponse::SettingsSetResponse
        )
    }

    pub fn set_settings(&mut self, scope: String, json: JsValue) -> Promise {
        let json = JsValue::into_serde(&json).expect("JSON.stringify is compatible with serde_json::Value");
        let fut = self._set_settings(scope, json);
        to_promise(fut)
    }

    fn _get_schema(&self, scope: String) -> impl Future<Output = ActyxOSResult<serde_json::Value>> + 'static {
        admin!(
            self.tx,
            AdminRequest::SettingsSchema { scope: scope.into() },
            AdminResponse::SettingsSchemaResponse
        )
    }
    pub fn get_schema(&mut self, scope: String) -> Promise {
        let fut = self._get_schema(scope);
        to_promise(fut)
    }
    fn _nodes_ls(&self) -> impl Future<Output = ActyxOSResult<NodesLsResponse>> + 'static {
        admin!(self.tx, AdminRequest::NodesLs, AdminResponse::NodesLsResponse)
    }
    pub fn nodes_ls(&mut self) -> Promise {
        let fut = self._nodes_ls();
        to_promise(fut)
    }

    fn _nodes_inspect(&self) -> impl Future<Output = ActyxOSResult<NodesInspectResponse>> + 'static {
        admin!(self.tx, AdminRequest::NodesInspect, AdminResponse::NodesInspectResponse)
    }

    pub fn nodes_inspect(&mut self) -> Promise {
        let fut = self._nodes_inspect();
        to_promise(fut)
    }
    // node manager functions. TODO: refactor into smaller function
    fn _get_node_details(&self) -> impl Future<Output = ActyxOSResult<ConnectedNodeDetails>> + 'static {
        let x = futures::future::try_join5(
            self._nodes_ls(),
            self._nodes_inspect(),
            self._get_settings("com.actyx".into()),
            self._get_schema("com.actyx".into()),
            self._offsets(),
        );

        async move {
            let (nodes_ls, nodes_inspect, settings, schema, offsets) = x.await?;
            let ret = ConnectedNodeDetails {
                node_id: nodes_ls.node_id,
                display_name: nodes_ls.display_name,
                started_iso: nodes_ls.started_iso,
                started_unix: nodes_ls.started_unix,
                version: format!("{}", nodes_ls.version),
                addrs: "TODO".into(),
                swarm_state: nodes_inspect,
                settings_schema: schema,
                settings,
                offsets: Some(offsets),
            };
            Ok(ret)
        }
    }
    pub fn get_node_details(&mut self) -> Promise {
        let fut = self._get_node_details();
        to_promise(async move {
            let addr = "FIXME".into();
            match fut.await {
                Err(e) if e.code() == ActyxOSCode::ERR_NODE_UNREACHABLE => {
                    eprintln!("returning unreachable node {}", addr);
                    Ok(Node::UnreachableNode { addr })
                }
                Err(e) if e.code() == ActyxOSCode::ERR_UNAUTHORIZED => Ok(Node::UnauthorizedNode { addr }),
                Ok(details) => Ok(Node::ReachableNode { addr, details }),
                Err(e) => {
                    eprintln!("error getting node details: {}", e);
                    Err(anyhow::anyhow!(e))
                }
            }
        })
    }
}

#[derive(Debug, From)]
enum OutEvent {
    Admin(StreamingResponseEvent<AdminProtocol>),
    Events(StreamingResponseEvent<EventsProtocol>),
    Ping(PingEvent),
}
#[derive(NetworkBehaviour)]
#[behaviour(event_process = false, out_event = "OutEvent")]
struct RequestBehaviour {
    admin_api: StreamingResponse<AdminProtocol>,
    events_api: StreamingResponse<EventsProtocol>,
    ping: Ping,
}

async fn run(private_key: &str, mut rx: mpsc::Receiver<Channel>) -> anyhow::Result<()> {
    let mut bytes = base64::decode(&private_key.as_bytes()[1..])?;
    let pri = identity::ed25519::SecretKey::from_bytes(&mut bytes[..])?;
    let kp = identity::ed25519::Keypair::from(pri);
    let mut swarm = mk_swarm(identity::Keypair::Ed25519(kp))?;
    let (remote_peer, remote_addr) = poll_until_connected(
        &mut swarm,
        std::iter::once("/ip4/127.0.0.1/tcp/4459/ws".parse().unwrap()),
    )
    .await?;
    info!("Connected to {} at {}", remote_peer, remote_addr);

    let mut pending_event_requests = BTreeMap::new();
    let mut pending_admin_requests = BTreeMap::new();
    loop {
        select! {
            request = rx.select_next_some() => {
                match request {
                    Either::Left((request, tx)) => {
                        let id = swarm.behaviour_mut().admin_api.request(remote_peer, request);
                        pending_admin_requests.insert(id, tx);
                    },
                    Either::Right((request, tx)) => {
                        let id = swarm.behaviour_mut().events_api.request(remote_peer, request);
                        pending_event_requests.insert(id, tx);
                    },
                }
            },
            ev = swarm.select_next_some() => {
                info!("Received {:?}", ev);
                match ev {
                    SwarmEvent::Behaviour(
                        OutEvent::Events(
                        StreamingResponseEvent::ResponseReceived{
                            request_id,
                            payload,
                            ..
                        })) => {
                        if let Some(tx) = pending_event_requests.get_mut(&request_id) {
                            if tx.start_send(Ok(payload)).is_err() {
                                pending_event_requests.remove(&request_id);
                                error!("FIXME");
                            }

                        }
                    },
                    SwarmEvent::Behaviour(
                        OutEvent::Events(
                        StreamingResponseEvent::ResponseFinished{
                            request_id,
                            ..
                        })) => {
                        pending_event_requests.remove(&request_id);
                    },
                    SwarmEvent::Behaviour(
                        OutEvent::Admin(
                        StreamingResponseEvent::ResponseReceived{
                            request_id,
                            payload,
                            ..
                        })) => {
                        if let Some(mut tx) = pending_admin_requests.remove(&request_id) {
                            if tx.start_send(payload).is_err() {
                                error!("FIXME");
                            }

                        }
                    }
                    SwarmEvent::Behaviour(
                        OutEvent::Admin(
                        StreamingResponseEvent::ResponseFinished{
                            request_id,
                            ..
                        })) => {
                        pending_admin_requests.remove(&request_id);
                    },


                    // TODO error handling
                    _ => {},
                }
            },
            complete => {
                error!("Stream ended!");
                break;
            }
        }
    }

    Ok(())
}

fn mk_swarm(kp: identity::Keypair) -> anyhow::Result<Swarm<RequestBehaviour>> {
    let peer_id: PeerId = kp.public().into();
    let transport = mk_transport(kp)?;

    let protocol = RequestBehaviour {
        admin_api: StreamingResponse::new(Default::default()),
        events_api: StreamingResponse::new(Default::default()),
        ping: Ping::new(PingConfig::new().with_keep_alive(true)),
    };
    let swarm = Swarm::new(transport, protocol, peer_id);
    Ok(swarm)
}

fn mk_transport(key_pair: identity::Keypair) -> anyhow::Result<Boxed<(PeerId, StreamMuxerBox)>> {
    let xx_keypair = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&key_pair)
        .unwrap();
    let noise_config = noise::NoiseConfig::xx(xx_keypair).into_authenticated();
    let yamux_config = YamuxConfig::default();
    let transport = ExtTransport::new(ffi::websocket_transport())
        .upgrade()
        .authenticate_with_version(noise_config, AuthenticationVersion::V1SimultaneousOpen)
        .multiplex(yamux_config)
        .timeout(Duration::from_secs(10))
        .map(|(peer_id, muxer), _| (peer_id, StreamMuxerBox::new(muxer)))
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
        .boxed();
    Ok(transport)
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
                info!("Listening on {}", address);
            }
            SwarmEvent::UnknownPeerUnreachableAddr { address, .. } | SwarmEvent::UnreachableAddr { address, .. } => {
                to_try -= 1;
                if to_try == 0 {
                    return ax_err(ActyxOSCode::ERR_NODE_UNREACHABLE, format!("{} is unreachable", address));
                } else {
                    info!(
                        "{} is unreachable, still got {} other connections to try",
                        address, to_try
                    );
                }
            }
            m => {
                info!("Uexpected message {:?}", m);
            }
        }
    }
    info!("Swarm exited unexpectedly");
    ax_err(ActyxOSCode::ERR_INTERNAL_ERROR, "Swarm exited unexpectedly".into())
}
