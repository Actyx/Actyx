use std::{collections::BTreeMap, io, time::Duration};

use actyx_sdk::service::OffsetsResponse;
use admin_protocol::{AdminProtocol, AdminResponse, ConnectedNodeDetails, NodesInspectResponse, NodesLsResponse};
use derive_more::From;
use errors::ActyxOSResult;
use events_protocol::{EventsProtocol, EventsRequest, EventsResponse};
use futures::{
    channel::{mpsc, oneshot},
    future::BoxFuture,
    select, Future, StreamExt,
};
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
use once_cell::sync::{Lazy, OnceCell};
use serde::Serialize;
use wasm_bindgen_futures::future_to_promise;

use crate::{
    admin_protocol::AdminRequest,
    errors::{ax_err, ActyxOSCode},
};
use log::{error, info};
use wasm_bindgen::JsValue;
use wasm_bindgen::{__rt::IntoJsResult, prelude::*};

mod admin_protocol;
mod errors;
mod events_protocol;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn main() {
    let _ = console_log::init_with_level(log::Level::Debug);
    ::console_error_panic_hook::set_once();
    info!("Setup panic hook");
}

//#[wasm_bindgen]
//pub async fn start() -> Result<(), JsValue> {
//    convert_result(run().await)
//}

fn convert_result<T, E: std::fmt::Debug>(result: std::result::Result<T, E>) -> Result<T, JsValue> {
    result.map_err(|err| js_sys::Error::new(&format!("WASM Internal Error: {:?}", err)).into())
}

fn map_err<T, E: Into<anyhow::Error>>(res: Result<T, E>) -> Result<T, JsValue> {
    res.map_err(Into::<anyhow::Error>::into)
        .map_err(|e| js_sys::Error::new(&format!("Error: {:#}", e)).into())
}
#[derive(Debug)]
enum Either<A, B> {
    Left(A),
    Right(B),
}

// TODO: handle streams
// TODO: handle multiple nodes!
type Channel = Either<(AdminRequest, oneshot::Sender<AdminResponse>), (EventsRequest, oneshot::Sender<EventsResponse>)>;
static SWARM: OnceCell<mpsc::Sender<Channel>> = OnceCell::new();

#[wasm_bindgen]
pub struct ActyxAdminApi {}

fn to_promise(fut: impl Future<Output = anyhow::Result<impl Serialize>> + 'static) -> Promise {
    future_to_promise(async move {
        fut.await
            //            .and_then(|x| JsValue::from_serde(&x).map_err(|x| anyhow::anyhow!("wtf"))
            .map(|e| JsValue::from_serde(&e).unwrap())
            .map_err(|e| js_sys::Error::new(&format!("Error: {:#}", e)).into())
    })
}

#[wasm_bindgen]
impl ActyxAdminApi {
    #[wasm_bindgen(constructor)]
    pub fn new(private_key: String) -> Self {
        let (tx, rx) = mpsc::channel(64);

        SWARM.set(tx).expect("Though shall not init twice!");
        // TODO: Move this to a webworker.
        // Right now, this basically just spawns the promise to wherever.
        future_to_promise(async move {
            // TODO: expose via public interface
            run(&*private_key, rx).await.unwrap();
            Ok("XX".into())
        });
        Self {}
    }

    fn _offsets(&self) -> impl Future<Output = anyhow::Result<OffsetsResponse>> + 'static {
        async move {
            let (tx, rx) = oneshot::channel();
            SWARM
                .get()
                .expect("struct created through `new`")
                .clone()
                .start_send(Either::Right((EventsRequest::Offsets, tx)))
                .unwrap();

            if let EventsResponse::Offsets(x) = rx.await? {
                Ok(x)
            } else {
                anyhow::bail!("Received unknown response")
            }
        }
    }
    // Unfortunately promises can't be typed, they always end up as `Promise<any>` in the ts
    // definition file. Synchronous function can be annotated with `#[wasm_bindgen(typescript_type = "..")]`
    pub fn offsets(&mut self) -> Promise {
        to_promise(self._offsets())
    }
    fn _get_settings(&self, scope: String) -> impl Future<Output = anyhow::Result<serde_json::Value>> + 'static {
        async move {
            let (tx, rx) = oneshot::channel();
            SWARM
                .get()
                .expect("struct created through `new`")
                .clone()
                .start_send(Either::Left((
                    AdminRequest::SettingsGet {
                        scope: scope.into(),
                        no_defaults: false,
                    },
                    tx,
                )))
                .unwrap();
            if let AdminResponse::SettingsGetResponse(x) = rx.await? {
                Ok(x)
            } else {
                anyhow::bail!("Received unknown response")
            }
        }
    }
    fn _get_schema(&self, scope: String) -> impl Future<Output = anyhow::Result<serde_json::Value>> + 'static {
        async move {
            let (tx, rx) = oneshot::channel();
            SWARM
                .get()
                .expect("struct created through `new`")
                .clone()
                .start_send(Either::Left((AdminRequest::SettingsSchema { scope: scope.into() }, tx)))
                .unwrap();

            if let AdminResponse::SettingsSchemaResponse(x) = rx.await? {
                Ok(x)
            } else {
                anyhow::bail!("Received unknown response")
            }
        }
    }
    fn _nodes_ls(&self) -> impl Future<Output = anyhow::Result<NodesLsResponse>> + 'static {
        async move {
            let (tx, rx) = oneshot::channel();
            SWARM
                .get()
                .expect("struct created through `new`")
                .clone()
                .start_send(Either::Left((AdminRequest::NodesLs, tx)))
                .unwrap();

            if let AdminResponse::NodesLsResponse(x) = rx.await? {
                Ok(x)
            } else {
                anyhow::bail!("Received unknown response")
            }
        }
    }
    pub fn nodes_ls(&mut self) -> Promise {
        let fut = self._nodes_ls();
        to_promise(fut)
    }
    fn _nodes_inspect(&self) -> impl Future<Output = anyhow::Result<NodesInspectResponse>> + 'static {
        async move {
            let (tx, rx) = oneshot::channel();
            SWARM
                .get()
                .expect("struct created through `new`")
                .clone()
                .start_send(Either::Left((AdminRequest::NodesInspect, tx)))
                .unwrap();

            if let AdminResponse::NodesInspectResponse(x) = rx.await? {
                Ok(x)
            } else {
                anyhow::bail!("Received unknown response")
            }
        }
    }
    pub fn nodes_inspect(&mut self) -> Promise {
        let fut = self._nodes_inspect();
        to_promise(fut)
    }
    // node manager functions. TODO: refactor into smaller function
    fn _get_node_details(&self) -> impl Future<Output = anyhow::Result<ConnectedNodeDetails>> + 'static {
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
        to_promise(self._get_node_details())
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
    let x = swarm
        .behaviour_mut()
        .admin_api
        .request(remote_peer, AdminRequest::NodesLs);

    let mut pending_requests = BTreeMap::new();
    loop {
        select! {
            request = rx.select_next_some() => {
                match request {
                    Either::Left(_) => {
                        todo!()
                    },
                    Either::Right((request, tx)) => {
                        let id = swarm.behaviour_mut().events_api.request(remote_peer, request);
                        pending_requests.insert(id, tx);
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
                        if let Some(tx) = pending_requests.remove(&request_id) {
                            if tx.send(payload).is_err() {
                                error!("FIXME");
                            }

                        }
                    },
                    // TODO
                    _ => {},
                }
            },
            complete => {
                error!("Stream ended unexpectidely!");
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
