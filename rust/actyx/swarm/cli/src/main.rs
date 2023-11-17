use acto::ActoRef;
use actyx_sdk::{app_id, service::SwarmState, AppId, Payload};
use anyhow::Result;
use axlib::{
    api::{self, formats::Licensing, NodeInfo},
    ax_futures_util::stream::AxStreamExt,
    crypto::{KeyPair, KeyStore},
    swarm::{
        blob_store::BlobStore,
        event_store_ref::{self, EventStoreHandler, EventStoreRef, EventStoreRequest},
        BanyanStore, DbPath, GossipMessage, SwarmConfig,
    },
    trees::{query::TagExprQuery, AxKey},
    util::variable::Writer,
};
use cbor_data::{
    codec::{CodecError, ReadCbor},
    Cbor,
};
use futures::{stream::StreamExt, FutureExt, TryStreamExt};
use ipfs_embed::GossipEvent;
use libp2p::PeerId;
use parking_lot::Mutex;
use std::sync::Arc;
use structopt::StructOpt;
use swarm_cli::{Command, Config, Event};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    runtime::Handle,
    sync::mpsc,
};
use tracing_subscriber::fmt::format::FmtSpan;

fn make_log_filename() -> String {
    std::env::vars()
        .find(|x| x.0 == "NETSIM_TEST_LOGFILE")
        .map(|x| x.1)
        .unwrap_or("unknown".to_string())
}

#[tokio::main]
async fn main() {
    let config = Config::from_args();

    tracing_log::LogTracer::init().ok();
    // install global collector configured based on RUST_LOG env var.

    let log_filename = make_log_filename();
    let file_appender =
        tracing_appender::rolling::minutely("./test-log/netsim/", format!("{}-{}.log", log_filename, config.keypair));
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .with_writer(std::io::stderr)
        .with_writer(non_blocking)
        .finish();

    tracing::subscriber::set_global_default(subscriber).ok();
    log_panics::init();

    axlib::util::setup_logger();
    if let Err(err) = run(config).await {
        tracing::error!("{}", err);
    }
}

async fn run(mut config: Config) -> Result<()> {
    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut line = String::with_capacity(4096);
    fn app_id() -> AppId {
        app_id!("com.actyx.swarm-cli")
    }

    tracing::info!(
        "mdns: {} fast_path: {} slow_path: {} root_map: {} discovery: {} metrics: {} api: {:?}",
        config.enable_mdns,
        config.enable_fast_path,
        config.enable_slow_path,
        config.enable_root_map,
        config.enable_discovery,
        config.enable_metrics,
        config.enable_api
    );
    let listen_addresses = std::mem::take(&mut config.listen_on);
    let swarm = if let Some(addr) = config.enable_api {
        let cfg = SwarmConfig::from(config.clone());
        let mut key_store = KeyStore::default();
        key_store.add_key_pair_ed25519(cfg.keypair.unwrap_or_else(KeyPair::generate).into())?;
        let swarm = BanyanStore::new(cfg, ActoRef::blackhole()).await?;
        tracing::info!("Binding api to {:?}", addr);
        let node_info = NodeInfo::new(
            swarm.node_id(),
            key_store.into_ref(),
            0.into(),
            Licensing::default(),
            chrono::Utc::now(),
        );
        let (tx, _rx) = crossbeam::channel::unbounded();
        let event_store = {
            let store = swarm.clone();
            let (tx, mut rx) = mpsc::channel::<EventStoreRequest>(10);
            swarm.spawn_task(
                "handler".to_owned(),
                async move {
                    let mut handler = EventStoreHandler::new(store);
                    let runtime = Handle::current();
                    while let Some(request) = rx.recv().await {
                        let req = request.to_string();
                        tracing::debug!("got request {}", req);
                        handler.handle(request, &runtime);
                        tracing::debug!("handled request {}", req);
                    }
                }
                .boxed(),
            );
            EventStoreRef::new(move |e| tx.try_send(e).map_err(event_store_ref::Error::from))
        };
        let blobs = BlobStore::new(DbPath::Memory)?;
        let swarm_state = Writer::new(SwarmState::default()).reader();
        swarm.spawn_task(
            "api".to_owned(),
            api::run(
                node_info,
                swarm.clone(),
                event_store,
                blobs,
                Arc::new(Mutex::new(addr.into())),
                tx,
                swarm_state,
            )
            .boxed(),
        );
        swarm
    } else {
        BanyanStore::new(config.clone().into(), ActoRef::blackhole()).await?
    };

    let mut ipfs = swarm.ipfs().clone();

    let mut stream = ipfs.swarm_events().await.unwrap();
    // make sure we don't lose `NewListenAddr` events.
    for listen_addr in listen_addresses {
        let mut stream = ipfs.listen_on(listen_addr);
        tokio::spawn(async move {
            while let Some(event) = stream.next().await {
                let event = match event {
                    ipfs_embed::ListenerEvent::NewListenAddr(addr) => Event::NewListenAddr(addr),
                    ipfs_embed::ListenerEvent::ExpiredListenAddr(addr) => Event::ExpiredListenAddr(addr),
                    ipfs_embed::ListenerEvent::ListenFailed(addr, reason) => Event::ListenFailed(addr, reason),
                };
                println!("{}", event);
            }
        });
    }

    // Poor man's fix for missing ipfs_embed::Event::Connected and
    // ipfs_embed::Event::ConnectionEstablished event from ipfs.swarm_events()
    tokio::spawn(async move {
        use std::collections::HashSet;
        use tokio::time::{sleep, Duration};
        let ipfs = ipfs.clone();
        let mut last_connected = HashSet::<PeerId>::new();
        loop {
            let current_connected = ipfs
                .peers()
                .into_iter()
                .filter(|peer| ipfs.is_connected(peer))
                .collect::<HashSet<PeerId>>();

            let new_connected = &current_connected - &last_connected;
            let new_disconnected = &last_connected - &current_connected;

            if !new_connected.is_empty() {
                tracing::info!(
                    "{} connected to: {}",
                    ipfs.local_peer_id(),
                    new_connected
                        .iter()
                        .map(|x| format!(" - {}", x))
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }
            if !new_disconnected.is_empty() {
                tracing::info!(
                    "{} disconnected from: {}",
                    ipfs.local_peer_id(),
                    new_disconnected
                        .iter()
                        .map(|x| format!(" - {}", x))
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }
            if new_connected.is_empty() && new_disconnected.is_empty() {
                tracing::info!("{} there is no connection status update", ipfs.local_peer_id());
            }

            new_connected.into_iter().for_each(|peer_id| {
                println!("{}", Event::Connected(peer_id));
            });

            new_disconnected.into_iter().for_each(|peer_id| {
                println!("{}", Event::Disconnected(peer_id));
            });

            last_connected = current_connected;

            sleep(Duration::from_millis(1000)).await;
        }
    });

    tokio::spawn(async move {
        while let Some(event) = stream.next().await {
            tracing::debug!("got event {:?}", event);
            let event = match event {
                ipfs_embed::Event::NewExternalAddr(addr) => Some(Event::NewExternalAddr(addr)),
                ipfs_embed::Event::ExpiredExternalAddr(addr) => Some(Event::ExpiredExternalAddr(addr)),
                ipfs_embed::Event::Discovered(peer_id) => Some(Event::Discovered(peer_id)),
                ipfs_embed::Event::Unreachable(peer_id) => Some(Event::Unreachable(peer_id)),
                // NOTE: ipfs_embed::Event::Connected is not always emitted
                // Therefore ipfs_embed::Event::ConnectionEstablished is used as a fallback
                // See:
                //  - https://docs.rs/crate/ipfs-embed/latest/source/src/net/peers.rs#:~:text=self.notify(Event%3A%3AConnected(c.peer_id))%3B
                //      Connected event is SOMETIMES emitted, that is only when `event.other_is_established == 0`.
                //  - https://docs.rs/crate/libp2p-swarm/0.41.1/source/src/lib.rs#:~:text=let%20non_banned_established
                //      other_established_connection_ids - banned_peers
                //  - https://docs.rs/crate/libp2p-swarm/0.41.1/source/src/lib.rs#:~:text=let%20non_banned_established
                //      other_established_connection_ids is the

                // Better not remove these reroutings of ConnectionEstablished,
                // Connected, and Disconnected despite having the above
                // poor-man's fix because some events can arrive but isn't
                // caught by the above loop because the loop might be too slow
                // to catch these events
                ipfs_embed::Event::ConnectionEstablished(peer_id, _) => Some(Event::Connected(peer_id)),
                ipfs_embed::Event::Connected(peer_id) => Some(Event::Connected(peer_id)),
                ipfs_embed::Event::Disconnected(peer_id) => Some(Event::Disconnected(peer_id)),
                ipfs_embed::Event::Subscribed(peer_id, topic) => Some(Event::Subscribed(peer_id, topic)),
                _ => None,
            };
            if let Some(event) = event {
                println!("{}", event);
            }
        }
    });

    let node_id = swarm.node_id();

    loop {
        line.clear();
        stdin.read_line(&mut line).await?;
        match line.parse()? {
            Command::AddAddress(peer, addr) => swarm.ipfs().clone().add_address(peer, addr),
            Command::Append(events) => {
                swarm.append(app_id(), events).await?;
            }
            Command::SubscribeQuery(q) => {
                let from = match q.source {
                    actyx_sdk::language::Source::Events { from, .. } => from,
                    actyx_sdk::language::Source::Array(_) => unimplemented!(),
                };
                let tags_query = TagExprQuery::from_expr(&from).unwrap();
                let this = swarm.clone();
                let mut stream = swarm
                    .stream_known_streams()
                    .map(move |stream_id| {
                        this.stream_filtered_chunked(
                            stream_id,
                            0..=u64::max_value(),
                            tags_query(stream_id.node_id() == node_id, stream_id),
                        )
                    })
                    .merge_unordered()
                    .map_ok(|chunk| futures::stream::iter(chunk.data).map(Ok))
                    .try_flatten();
                tokio::spawn(async move {
                    while let Some(res) = stream.next().await as Option<anyhow::Result<(u64, AxKey, Payload)>> {
                        println!("{}", Event::Result(res.unwrap()));
                    }
                });
            }
            Command::ApiPort => {
                println!("{}", Event::ApiPort(config.enable_api.map(|a| a.port())));
            }
            Command::GossipSubscribe(topic) => {
                let mut stream = swarm.ipfs().clone().subscribe(topic.clone()).await?;
                tokio::spawn(async move {
                    while let Some(msg) = stream.next().await {
                        if let GossipEvent::Message(sender, data) = msg {
                            match Cbor::checked(&data[..])
                                .map_err(CodecError::custom)
                                .and_then(GossipMessage::read_cbor)
                            {
                                Ok(x) => {
                                    println!("{}", Event::GossipEvent(topic.clone(), sender, x));
                                }
                                Err(e) => {
                                    println!("Error decoding GossipMessage: {}", e);
                                }
                            }
                        }
                    }
                });
            }
        }
    }
}
