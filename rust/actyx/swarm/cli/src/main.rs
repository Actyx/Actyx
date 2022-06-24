use actyx_sdk::{app_id, AppId, Payload};
use anyhow::Result;
use api::{formats::Licensing, NodeInfo};
use ax_futures_util::prelude::AxStreamExt;
use crypto::{KeyPair, KeyStore};
use futures::{stream::StreamExt, TryStreamExt};
use ipfs_embed::GossipEvent;
use libipld::{cbor::DagCborCodec, codec::Codec};
use parking_lot::Mutex;
use std::sync::Arc;
use structopt::StructOpt;
use swarm::{
    blob_store::BlobStore,
    event_store_ref::{self, EventStoreHandler, EventStoreRef, EventStoreRequest},
    BanyanStore, DbPath, GossipMessage, SwarmConfig,
};
use swarm_cli::{Command, Config, Event};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    runtime::Handle,
    sync::mpsc,
};
use trees::{query::TagExprQuery, AxKey};

#[tokio::main]
async fn main() {
    util::setup_logger();
    if let Err(err) = run().await {
        tracing::error!("{}", err);
    }
}

async fn run() -> Result<()> {
    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut line = String::with_capacity(4096);
    fn app_id() -> AppId {
        app_id!("com.actyx.swarm-cli")
    }

    let mut config = Config::from_args();
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
        let swarm = BanyanStore::new(cfg).await?;
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
            swarm.spawn_task("handler", async move {
                let mut handler = EventStoreHandler::new(store);
                let runtime = Handle::current();
                while let Some(request) = rx.recv().await {
                    let req = request.to_string();
                    tracing::debug!("got request {}", req);
                    handler.handle(request, &runtime);
                    tracing::debug!("handled request {}", req);
                }
            });
            EventStoreRef::new(move |e| tx.try_send(e).map_err(event_store_ref::Error::from))
        };
        let blobs = BlobStore::new(DbPath::Memory)?;
        swarm.spawn_task(
            "api",
            api::run(
                node_info,
                swarm.clone(),
                event_store,
                blobs,
                Arc::new(Mutex::new(addr.into())),
                tx,
            ),
        );
        swarm
    } else {
        BanyanStore::new(config.clone().into()).await?
    };
    let mut stream = swarm.ipfs().swarm_events();
    // make sure we don't lose `NewListenAddr` events.
    for listen_addr in listen_addresses {
        let _ = swarm.ipfs().listen_on(listen_addr).unwrap();
    }
    tokio::spawn(async move {
        while let Some(event) = stream.next().await {
            tracing::debug!("got event {:?}", event);
            let event = match event {
                ipfs_embed::Event::NewListenAddr(_, addr) => Some(Event::NewListenAddr(addr)),
                ipfs_embed::Event::ExpiredListenAddr(_, addr) => Some(Event::ExpiredListenAddr(addr)),
                ipfs_embed::Event::NewExternalAddr(addr) => Some(Event::NewExternalAddr(addr)),
                ipfs_embed::Event::ExpiredExternalAddr(addr) => Some(Event::ExpiredExternalAddr(addr)),
                ipfs_embed::Event::Discovered(peer_id) => Some(Event::Discovered(peer_id)),
                ipfs_embed::Event::Unreachable(peer_id) => Some(Event::Unreachable(peer_id)),
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
            Command::AddAddress(peer, addr) => swarm.ipfs().add_address(&peer, addr),
            Command::Append(nr, events) => {
                swarm.append(nr, app_id(), events).await?;
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
                let mut stream = swarm.ipfs().subscribe(&topic)?;
                tokio::spawn(async move {
                    while let Some(msg) = stream.next().await {
                        if let GossipEvent::Message(sender, data) = msg {
                            match DagCborCodec.decode::<GossipMessage>(&data[..]) {
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
