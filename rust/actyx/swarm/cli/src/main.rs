use anyhow::Result;
use futures::stream::StreamExt;
use structopt::StructOpt;
use swarm::BanyanStore;
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

    let config = Config::from_args();
    tracing::info!(
        "mdns: {} fast_path: {} slow_path: {} root_map: {} discovery: {} metrics: {}",
        config.enable_mdns,
        config.enable_fast_path,
        config.enable_slow_path,
        config.enable_root_map,
        config.enable_discovery,
        config.enable_metrics,
    );
    let swarm = BanyanStore::new(config.into()).await?;
    println!("{}", Event::PeerId(swarm.ipfs().local_peer_id()));
    let mut stream = swarm.ipfs().swarm_events();
    tokio::spawn(async move {
        while let Some(event) = stream.next().await {
            let event = match event {
                ipfs_embed::Event::Connected(peer_id) => Some(Event::Connected(peer_id)),
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
            Command::Query(q) => {
                let tags_query = TagsQuery::from_expr(&q.from, true).unwrap();
                let mut stream = swarm.stream_filtered_stream_ordered(tags_query);
                tokio::spawn(async move {
                    while let Some(res) = stream.next().await {
                        println!("{}", Event::Result(res.unwrap()));
                    }
                });
            }
            Command::Exit => {
                break;
            }
        }
    }
    Ok(())
}
