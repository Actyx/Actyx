use anyhow::Result;
use futures::stream::StreamExt;
use std::io::Write;
use structopt::StructOpt;
use swarm::BanyanStore;
use swarm_cli::{Command, Config, Event};

#[tokio::main]
async fn main() {
    util::setup_logger();
    if let Err(err) = run().await {
        tracing::error!("{}", err);
    }
}

async fn run() -> Result<()> {
    let stdin = std::io::stdin();
    let mut line = String::with_capacity(4096);

    let swarm = BanyanStore::new(Config::from_args().into()).await?;

    loop {
        line.clear();
        stdin.read_line(&mut line)?;
        match line.parse()? {
            Command::Append(nr, events) => {
                swarm.append(nr, events).await?;
            }
            Command::Query(expr) => {
                let mut stream = swarm.stream_filtered_stream_ordered(expr.dnf());
                tokio::spawn(async move {
                    let mut stdout = std::io::stdout();
                    while let Some(res) = stream.next().await {
                        writeln!(stdout, "{}", Event::Result(res.unwrap())).unwrap();
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
