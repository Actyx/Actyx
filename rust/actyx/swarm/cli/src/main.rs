use actyxos_sdk::NodeId;
use anyhow::Result;
use api::NodeInfo;
use crypto::KeyStoreRef;
use futures::stream::StreamExt;
use structopt::StructOpt;
use swarm::{BanyanStore, SwarmConfig};
use swarm_cli::{Command, Config, Event};
use tokio::io::{AsyncBufReadExt, BufReader};
use trees::query::TagsQuery;

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
    let listen_addresses = std::mem::replace(&mut config.listen_on, vec![]);
    let swarm = if let Some(addr) = config.enable_api {
        let key_store: KeyStoreRef = Default::default();
        let public_key = key_store.write().generate_key_pair()?;
        let keypair = key_store.read().get_pair(public_key).unwrap();
        let node_id: NodeId = public_key.into();
        let mut swarm_config: SwarmConfig = config.into();
        swarm_config.keypair = Some(keypair);
        let swarm = BanyanStore::new(swarm_config).await?;
        tracing::info!("Binding api to {:?}", addr);
        let node_info = NodeInfo {
            node_id,
            key_store,
            token_validity: u32::MAX,
            cycles: 0.into(),
        };
        swarm.spawn_task("api", api::run(node_info, swarm.clone(), std::iter::once(addr)));
        swarm
    } else {
        BanyanStore::new(config.into()).await?
    };
    let mut stream = swarm.ipfs().swarm_events();
    // make sure we don't lose `NewListenAddr` events.
    for listen_addr in listen_addresses {
        let _ = swarm.ipfs().listen_on(listen_addr).unwrap();
    }
    tokio::spawn(async move {
        while let Some(event) = stream.next().await {
            let event = match event {
                ipfs_embed::Event::NewListenAddr(_, addr) => Some(Event::NewListenAddr(addr)),
                ipfs_embed::Event::ExpiredListenAddr(_, addr) => Some(Event::ExpiredListenAddr(addr)),
                ipfs_embed::Event::NewExternalAddr(addr) => Some(Event::NewExternalAddr(addr)),
                ipfs_embed::Event::ExpiredExternalAddr(addr) => Some(Event::ExpiredExternalAddr(addr)),
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

    loop {
        line.clear();
        stdin.read_line(&mut line).await?;
        match line.parse()? {
            Command::AddAddress(peer, addr) => swarm.ipfs().add_address(&peer, addr),
            Command::Append(nr, events) => {
                swarm.append(nr, events).await?;
            }
            Command::SubscribeQuery(q) => {
                let tags_query = TagsQuery::from_expr(&q.from)(true);
                let mut stream = swarm.stream_filtered_stream_ordered(tags_query);
                tokio::spawn(async move {
                    while let Some(res) = stream.next().await {
                        println!("{}", Event::Result(res.unwrap()));
                    }
                });
            }
        }
    }
}
