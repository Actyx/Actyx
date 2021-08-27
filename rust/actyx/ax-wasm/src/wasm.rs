use crate::types::{ConnectedNodeDetails, EventDiagnostic, Node, NodeManagerEventsRes};
use actyx_sdk::service::{self as sdk, Order, QueryRequest};
use crypto::PrivateKey;
use derive_more::From;
use futures::{channel::mpsc, pin_mut, select, Future, StreamExt};
use js_sys::Promise;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed, upgrade::AuthenticationVersion, ConnectedPoint},
    identity, noise,
    ping::{Ping, PingConfig, PingEvent},
    swarm::{SwarmBuilder, SwarmEvent},
    wasm_ext::{ffi, ExtTransport},
    yamux::YamuxConfig,
    Multiaddr, NetworkBehaviour, PeerId, Swarm, Transport,
};
use libp2p_streaming_response::{RequestId, StreamingResponse, StreamingResponseEvent};
use log::*;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::Serialize;
use std::{collections::BTreeMap, io, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};
use util::formats::{
    ax_err,
    events_protocol::{EventsProtocol, EventsRequest, EventsResponse},
    ActyxOSCode, ActyxOSResult, AdminProtocol, AdminRequest, AdminResponse, NodesInspectResponse, NodesLsResponse,
};
use wasm_bindgen::JsValue;
use wasm_bindgen::{prelude::*, JsCast};
use wasm_bindgen_futures::future_to_promise;
use wasm_futures_executor::ThreadPool;

#[wasm_bindgen(start)]
pub fn main() {
    let _ = console_log::init_with_level(log::Level::Info);
    ::console_error_panic_hook::set_once();
    debug!("Setup panic hook");
}

#[wasm_bindgen]
pub fn create_private_key() -> String {
    let key = PrivateKey::generate();
    key.to_string()
}

#[wasm_bindgen]
pub fn validate_private_key(input: String) -> std::result::Result<(), JsValue> {
    PrivateKey::from_str(&*input).map_err(|e| JsValue::from_str(&*format!("{:#}", e)))?;
    Ok(())
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
#[allow(clippy::type_complexity)]
static SWARMS: Lazy<Mutex<BTreeMap<String, Arc<Mutex<mpsc::Sender<Channel>>>>>> = Lazy::new(Default::default);
static THREAD_POOL: Lazy<Mutex<ThreadPool>> = Lazy::new(|| {
    let tp = ThreadPool::new(2).unwrap();
    Mutex::new(tp)
});

#[wasm_bindgen]
pub struct ActyxAdminApi {
    host: String,
    tx: Arc<Mutex<mpsc::Sender<Channel>>>,
}

fn to_promise(
    fut: impl Future<Output = std::result::Result<impl Serialize, impl std::fmt::Display>> + 'static,
) -> Promise {
    future_to_promise(async move {
        fut.await
            .map(|e| JsValue::from_serde(&e).expect("Serialize trait bound"))
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
fn to_multiaddr(input: &str) -> anyhow::Result<Multiaddr> {
    let s = if let Ok(socket) = input.parse::<SocketAddr>() {
        socket
    } else {
        format!("{}:4458", input).parse()?
    };
    Ok(match s {
        SocketAddr::V4(v4) => format!("/ip4/{}/tcp/{}/ws", v4.ip(), v4.port()),
        SocketAddr::V6(v6) => format!("/ip6/{}/tcp/{}/ws", v6.ip(), v6.port()),
    }
    .parse()?)
}

#[wasm_bindgen(typescript_custom_section)]
const ADDITIONAL_TYLES: &'static str = r#"
export type OffsetsResponse = { present: { [stream_id: string]: number }, toReplicate: { [stream_id: string]: number } }
"#;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "(events: EventDiagnostic[], err?: Error) => void")]
    pub type QueryCallback;
    #[wasm_bindgen(typescript_type = "Promise<OffsetsResponse>")]
    pub type PromiseOffsetsResponse;
}

#[wasm_bindgen]
impl ActyxAdminApi {
    #[wasm_bindgen(constructor)]
    #[allow(unused_must_use)]
    pub fn new(host: String, private_key: String) -> Self {
        // TODO: validate and convert private key
        // FIXME remove unwrap
        let addr = to_multiaddr(&*host).expect("Invalid input");

        let tx = SWARMS
            .lock()
            .entry(host.clone())
            .or_insert_with(|| {
                let (tx, rx) = mpsc::channel(64);

                // TODO: Move this to a webworker.
                // Right now, this basically just spawns the promise to wherever.
                //                future_to_promise(async move {
                //
                //                    run(addr, &*private_key, rx).await.unwrap();
                //                    Ok("XX".into())
                //                });
                THREAD_POOL.lock().spawn_ok(async move {
                    run(addr, &*private_key, rx).await.unwrap();
                });

                Arc::new(Mutex::new(tx))
            })
            .clone();
        Self { host, tx }
    }

    // Events API
    fn _offsets(&self) -> impl Future<Output = ActyxOSResult<sdk::OffsetsResponse>> + 'static {
        events!(self.tx, EventsRequest::Offsets, EventsResponse::Offsets)
    }
    // Unfortunately promises can't be typed, they always end up as `Promise<any>` in the ts
    // definition file. Synchronous function can be annotated with `#[wasm_bindgen(typescript_type = "..")]`
    // TODO: https://github.com/rustwasm/wasm-bindgen/pull/2665 just came in!
    pub fn offsets(&mut self) -> PromiseOffsetsResponse {
        let x = to_promise(self._offsets());
        JsCast::unchecked_into(x)
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

    pub fn query_cb(&mut self, query: String, cb: QueryCallback) {
        let cb: js_sys::Function = JsCast::unchecked_into(cb);
        let x = move |event: Option<ActyxOSResult<Vec<EventDiagnostic>>>| match event {
            Some(Ok(e)) => cb.call1(&JsValue::null(), &JsValue::from_serde(&e).expect("valid json")),
            Some(Err(e)) => cb.call2(
                &JsValue::null(),
                &JsValue::null(),
                &JsValue::from_str(&*format!("{:#}", e)),
            ),
            None => cb.call1(&JsValue::null(), &JsValue::null()),
        };
        let mut swarm_tx = self.tx.lock().clone();
        let fut = async move {
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

            let stream = rx
                .filter_map(|x| async move {
                    match x {
                        Ok(EventsResponse::Diagnostic(d)) => Some(Ok(EventDiagnostic::Diagnostic(d))),
                        Ok(EventsResponse::Event(e)) => Some(Ok(EventDiagnostic::Event(e))),
                        Err(e) => Some(Err(e)),
                        _ => None,
                    }
                })
                .ready_chunks(50);
            pin_mut!(stream);
            while let Some(res) = stream.next().await {
                let _ = x(Some(res.into_iter().collect::<ActyxOSResult<Vec<_>>>()));
                futures_timer::Delay::new(Duration::from_millis(500)).await;
            }
            // Signal end of stream
            let _ = x(None);
            Result::Ok::<_, anyhow::Error>(())
        };
        let _ = to_promise(fut);
    }

    // Admin API
    fn _get_settings(&self, scope: String) -> impl Future<Output = ActyxOSResult<serde_json::Value>> + 'static {
        admin!(
            self.tx,
            AdminRequest::SettingsGet {
                scope: to_scope(&*scope),
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
                scope: to_scope(&*scope),
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
            AdminRequest::SettingsSchema {
                scope: to_scope(&*scope)
            },
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

// this must not fail
async fn run(addr: Multiaddr, private_key: &str, mut rx: mpsc::Receiver<Channel>) -> anyhow::Result<()> {
    let mut bytes = base64::decode(&private_key.as_bytes()[1..])?;
    let pri = identity::ed25519::SecretKey::from_bytes(&mut bytes[..])?;
    let kp = identity::ed25519::Keypair::from(pri);
    let mut swarm = mk_swarm(identity::Keypair::Ed25519(kp))?;

    let mut connected_to = None;
    let mut pending_admin_requests: BTreeMap<RequestId, mpsc::Sender<ActyxOSResult<AdminResponse>>> =
        Default::default();
    let mut pending_event_requests: BTreeMap<RequestId, mpsc::Sender<ActyxOSResult<EventsResponse>>> =
        Default::default();
    loop {
        if connected_to.is_none() {
            // Retry connection after a connection failure. After the retries, the loop will sit in
            // the `futures::select` below waiting for new input. After new input has been
            // received, a new connection attempt is started.
            if let Err(e) = {
                let mut tries = 3;
                loop {
                    match poll_until_connected(&mut swarm, std::iter::once(addr.clone())).await {
                        Err(e) if tries > 0 => {
                            tries -= 1;
                            warn!("Error connecting to {}: {}. Retrying {} more times.", addr, e, tries);
                            futures_timer::Delay::new(Duration::from_secs(tries * 2)).await;
                        }
                        Ok((remote_peer, remote_addr)) => {
                            info!("Connected to {} at {}", remote_peer, remote_addr);
                            connected_to.replace(remote_peer);
                            break Ok((remote_peer, remote_addr));
                        }
                        o => break o,
                    }
                }
            } {
                error!("Error reconnecting to {}", addr);
                pending_event_requests.retain(|_, tx| {
                    let _ = tx.start_send(Err(e.clone()));
                    false
                });
                pending_admin_requests.retain(|_, tx| {
                    let _ = tx.start_send(Err(e.clone()));
                    false
                });
            }
        }
        select! {
            x = rx.next() => {
                if let Some(request) = x {
                    match request {
                        Either::Left((request, mut tx)) => {
                            if let Some(p) = connected_to {
                                let id = swarm.behaviour_mut().admin_api.request(p, request);
                                pending_admin_requests.insert(id, tx);
                            } else {
                                let _ = tx.start_send(Err(ActyxOSCode::ERR_NODE_UNREACHABLE.with_message("")));
                            }
                        },
                        Either::Right((request, mut tx)) => {
                            if let Some(p) = connected_to {
                                let id = swarm.behaviour_mut().events_api.request(p, request);
                                pending_event_requests.insert(id, tx);
                            } else {
                                let _ = tx.start_send(Err(ActyxOSCode::ERR_NODE_UNREACHABLE.with_message("")));
                            }
                        },
                    }
                } else {
                    info!("Receiver dropped, disconnecting ..");
                    break;
                }
            },
            ev = swarm.select_next_some() => {
                debug!("Received {:?}", ev);
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
                                swarm.behaviour_mut().events_api.cancel_request(request_id);
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
                        // all admin requests are oneshot
                        if let Some(mut tx) = pending_admin_requests.remove(&request_id) {
                            let _ =  tx.start_send(payload).is_err();
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
                    SwarmEvent::ConnectionClosed {
                        peer_id,
                        num_established,
                        cause,
                        ..
                    } => {
                        error!("Connection to {} closed: {:?}", peer_id, cause);
                        if num_established == 0 {
                            connected_to = None;
                        }
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

// FIXME move ThreadPool into global
fn mk_swarm(kp: identity::Keypair) -> anyhow::Result<Swarm<RequestBehaviour>> {
    let peer_id: PeerId = kp.public().into();
    let transport = mk_transport(kp)?;

    let protocol = RequestBehaviour {
        admin_api: StreamingResponse::new(Default::default()),
        events_api: StreamingResponse::new(Default::default()),
        ping: Ping::new(PingConfig::new().with_keep_alive(true)),
    };
    let builder = SwarmBuilder::new(transport, protocol, peer_id);
    Ok(builder.build())
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

// This is a dirty hack to get around the `settings` dependency when targeting wasm, but have the
// code compile successfully with the default target set.
#[cfg(not(target_arch = "wasm32"))]
fn to_scope(_: &str) -> settings::Scope {
    unreachable!()
}
#[cfg(target_arch = "wasm32")]
fn to_scope(s: &str) -> String {
    s.into()
}
