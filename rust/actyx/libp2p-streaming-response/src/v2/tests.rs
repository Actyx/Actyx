use super::{ProtocolError, StreamingResponse, StreamingResponseConfig};
use crate::{
    v2::{handler::Response, RequestReceived},
    Codec,
};
use futures::{
    channel::mpsc::{self, Receiver, Sender},
    Future, FutureExt, SinkExt, StreamExt,
};
use libp2p::{
    core::{transport::MemoryTransport, upgrade::Version},
    identity::Keypair,
    mplex::MplexConfig,
    multiaddr::Protocol,
    plaintext::PlainText2Config,
    swarm::{SwarmBuilder, SwarmEvent},
    Multiaddr, PeerId, Swarm, Transport,
};
use tokio::runtime::{Handle, Runtime};
use tracing_subscriber::{fmt::format::FmtSpan, util::SubscriberInitExt, EnvFilter};

mod proto;

const PROTO: &[u8] = b"/my/test";

fn test_swarm(use_spawner: Option<Handle>) -> Swarm<StreamingResponse<Proto>> {
    let local_key = Keypair::generate_ed25519();
    let local_public_key = local_key.public();
    let local_peer_id = local_public_key.clone().into();
    let transport = MemoryTransport::default()
        .upgrade(Version::V1)
        .authenticate(PlainText2Config { local_public_key })
        .multiplex(MplexConfig::new())
        .boxed();
    let mut config = StreamingResponseConfig::default()
        .with_keep_alive(true)
        .with_max_message_size(100);
    if let Some(rt) = use_spawner {
        config = config.with_spawner(move |f| {
            rt.spawn(f)
                .map(|r| r.unwrap_or_else(|e| Err(ProtocolError::JoinError(e.is_cancelled()))))
                .boxed()
        });
    }
    let behaviour = StreamingResponse::new(config);
    SwarmBuilder::new(transport, behaviour, local_peer_id).build()
}

fn fake_swarm(rt: &Runtime, bytes: &[u8]) -> Swarm<proto::TestBehaviour> {
    let local_key = Keypair::generate_ed25519();
    let local_public_key = local_key.public();
    let local_peer_id = local_public_key.clone().into();
    let transport = MemoryTransport::default()
        .upgrade(Version::V1)
        .authenticate(PlainText2Config { local_public_key })
        .multiplex(MplexConfig::new())
        .boxed();
    let behaviour = proto::TestBehaviour(rt.handle().clone(), bytes.to_owned());
    SwarmBuilder::new(transport, behaviour, local_peer_id).build()
}

struct Proto;
impl Codec for Proto {
    type Request = String;
    type Response = String;

    fn protocol_info() -> &'static [u8] {
        PROTO
    }
}

macro_rules! wait4 {
    ($s:ident, $p:pat => $e:expr) => {
        loop {
            let ev = $s.next().await;
            if ev.is_none() {
                panic!("{} STOPPED", stringify!($s))
            }
            let ev = ev.unwrap();
            log::info!("{} got {:?}", stringify!($s), ev);
            if let $p = ev {
                break $e;
            }
        }
    };
}

macro_rules! task {
    ($s:ident $(, $p:pat => $e:expr)*) => {
        tokio::spawn(async move {
            while let Some(ev) = $s.next().await {
                log::info!("{} got {:?}", stringify!($s), ev);
                match ev {
                    $($p => ($e),)*
                    _ => {}
                }
            }
            log::info!("{} STOPPED", stringify!($s));
        })
    };
}

fn dbg<T: std::fmt::Debug>(x: T) -> String {
    format!("{:?}", x)
}

fn setup_logger() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .finish()
        .try_init()
        .ok();
}

#[test]
fn smoke() {
    setup_logger();
    let rt = Runtime::new().unwrap();
    let mut asker = test_swarm(None);
    let asker_id = *asker.local_peer_id();
    let mut responder = test_swarm(None);
    let responder_id = *responder.local_peer_id();

    asker.listen_on(Multiaddr::empty().with(Protocol::Memory(0))).unwrap();

    rt.block_on(async move {
        let addr = wait4!(asker, SwarmEvent::NewListenAddr { address, .. } => address);

        responder.dial(addr).unwrap();
        task!(responder,
            SwarmEvent::Behaviour(RequestReceived { request, peer_id, mut channel, .. }) => {
                tokio::spawn(async move {
                    channel.feed(request).await.unwrap();
                    channel.feed(peer_id.to_string()).await.unwrap();
                    channel.close().await.unwrap();
                });
            }
        );

        let peer_id = wait4!(asker, SwarmEvent::ConnectionEstablished { peer_id, .. } => peer_id);
        assert_eq!(peer_id, responder_id);

        let (tx, rx) = mpsc::channel(10);
        asker.behaviour_mut().request(peer_id, "request".to_owned(), tx);

        task!(asker);

        let response = rx
            .map(|r| match r {
                Response::Msg(m) => Some(m),
                Response::Error(e) => panic!("got error: {:#}", e),
                Response::Finished => None,
            })
            .collect::<Vec<_>>()
            .await;
        assert_eq!(
            response,
            vec![Some("request".to_owned()), Some(asker_id.to_string()), None]
        );
    });
}

#[test]
fn smoke_executor() {
    setup_logger();
    let rt = Runtime::new().unwrap();
    let mut asker = test_swarm(Some(rt.handle().clone()));
    let asker_id = *asker.local_peer_id();
    let mut responder = test_swarm(Some(rt.handle().clone()));
    let responder_id = *responder.local_peer_id();

    asker.listen_on(Multiaddr::empty().with(Protocol::Memory(0))).unwrap();

    rt.block_on(async move {
        let addr = wait4!(asker, SwarmEvent::NewListenAddr { address, .. } => address);

        responder.dial(addr).unwrap();
        task!(responder,
            SwarmEvent::Behaviour(RequestReceived { request, peer_id, mut channel, .. }) => {
                tokio::spawn(async move {
                    channel.feed(request).await.unwrap();
                    channel.feed(peer_id.to_string()).await.unwrap();
                    channel.close().await.unwrap();
                });
            }
        );

        let peer_id = wait4!(asker, SwarmEvent::ConnectionEstablished { peer_id, .. } => peer_id);
        assert_eq!(peer_id, responder_id);

        let (tx, rx) = mpsc::channel(10);
        asker.behaviour_mut().request(peer_id, "request".to_owned(), tx);

        task!(asker);

        let response = rx
            .map(|r| match r {
                Response::Msg(m) => Some(m),
                Response::Error(e) => panic!("got error: {:#}", e),
                Response::Finished => None,
            })
            .collect::<Vec<_>>()
            .await;
        assert_eq!(
            response,
            vec![Some("request".to_owned()), Some(asker_id.to_string()), None]
        );
    });
}

fn test_setup<F, Fut, L>(request: String, logic: L, f: F)
where
    F: FnOnce(Receiver<Response<String>>) -> Fut + Send + 'static,
    Fut: Future,
    L: Fn(String, PeerId, Sender<String>) + Send + 'static,
{
    setup_logger();
    let rt = Runtime::new().unwrap();
    let mut asker = test_swarm(None);
    let mut responder = test_swarm(None);

    rt.block_on(async move {
        responder
            .listen_on(Multiaddr::empty().with(Protocol::Memory(0)))
            .unwrap();
        let addr = wait4!(responder, SwarmEvent::NewListenAddr{ address, .. } => address);
        task!(responder, SwarmEvent::Behaviour(RequestReceived { request, peer_id, channel, .. }) => logic(request, peer_id, channel));
        asker.dial(addr).unwrap();
        let peer_id = wait4!(asker, SwarmEvent::ConnectionEstablished { peer_id, .. } => peer_id);
        let (tx, rx) = mpsc::channel(10);
        asker.behaviour_mut().request(peer_id, request, tx);
        task!(asker);
        f(rx).await;
    });
}

fn fake_setup<F, Fut>(bytes: &[u8], f: F)
where
    F: FnOnce(Receiver<Response<String>>) -> Fut + Send + 'static,
    Fut: Future,
{
    setup_logger();
    let rt = Runtime::new().unwrap();
    let mut asker = test_swarm(None);
    let mut responder = fake_swarm(&rt, bytes);

    rt.block_on(async move {
        responder
            .listen_on(Multiaddr::empty().with(Protocol::Memory(0)))
            .unwrap();
        let addr = wait4!(responder, SwarmEvent::NewListenAddr{ address, .. } => address);
        task!(responder);
        asker.dial(addr).unwrap();
        let peer_id = wait4!(asker, SwarmEvent::ConnectionEstablished { peer_id, .. } => peer_id);
        let (tx, rx) = mpsc::channel(10);
        asker.behaviour_mut().request(peer_id, "request".to_owned(), tx);
        task!(asker);
        f(rx).await;
    });
}

#[test]
fn err_size() {
    fake_setup(b"zzzz", |mut rx| async move {
        assert_eq!(
            rx.next().await,
            Some(Response::Error(ProtocolError::MessageTooLargeRecv(2054847098)))
        );
    });
}

#[test]
fn err_nothing() {
    fake_setup(b"", |mut rx| async move {
        assert_eq!(dbg(rx.next().await.unwrap()), "Error(Io(Kind(UnexpectedEof)))");
    });
}

#[test]
fn err_incomplete() {
    fake_setup(b"\0\0\0\x05dabcd\0\0\0\x10abcd", |mut rx| async move {
        assert_eq!(rx.next().await, Some(Response::Msg("abcd".to_owned())));
        assert_eq!(dbg(rx.next().await.unwrap()), "Error(Io(Kind(UnexpectedEof)))");
    });
}

#[test]
fn err_no_finish() {
    fake_setup(b"\0\0\0\x05dabcd", |mut rx| async move {
        assert_eq!(rx.next().await, Some(Response::Msg("abcd".to_owned())));
        assert_eq!(dbg(rx.next().await.unwrap()), "Error(Io(Kind(UnexpectedEof)))");
    });
}

#[test]
fn err_deser() {
    fake_setup(b"\0\0\0\x04abcd", |mut rx| async move {
        assert_eq!(
            dbg(rx.next().await),
            "Some(Error(Serde(ErrorImpl { code: TrailingData, offset: 3 })))"
        );
    });
}

#[test]
fn err_response_size() {
    test_setup(
        "123456789012345678901234567890123456789012345678901234567890".to_owned(),
        |mut request, peer_id, mut channel| {
            tokio::spawn(async move {
                request.push_str(&*peer_id.to_string());
                channel.feed(request).await.unwrap();
            });
        },
        |mut rx| async move {
            assert_eq!(
                rx.next().await,
                Some(Response::Error(ProtocolError::MessageTooLargeSent(0)))
            );
        },
    );
}

#[test]
fn err_request_size() {
    test_setup(
        "1234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890"
            .to_owned(),
        |mut request, peer_id, mut channel| {
            tokio::spawn(async move {
                request.push_str(&*peer_id.to_string());
                channel.feed(request).await.unwrap();
            });
        },
        |mut rx| async move {
            assert_eq!(
                rx.next().await,
                Some(Response::Error(ProtocolError::MessageTooLargeSent(102)))
            );
        },
    );
}
