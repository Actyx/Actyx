use actyxos_sdk::{
    service::{EventService, Order, PublishEvent, PublishRequest, QueryRequest, QueryResponse},
    tags, HttpClient, Payload,
};
use assert_cmd::prelude::CommandCargoExt;
use futures::StreamExt;
use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use tempfile::tempdir;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;

async fn start_node(id: &str, working_dir: impl AsRef<Path>) -> anyhow::Result<(Child, HttpClient)> {
    let mut node = {
        let mut cmd = Command::cargo_bin("actyx-linux")?;
        cmd.args(&[
            "--working-dir",
            &*working_dir.as_ref().display().to_string(),
            "--bind-admin",
            "0",
            "--bind-api",
            "0",
            "--bind-swarm",
            "0",
        ])
        .env("RUST_LOG", "DEBUG");
        cmd
    };
    let mut node_handle = node.stdout(Stdio::piped()).spawn()?;
    let buf_reader = BufReader::new(node_handle.stdout.take().unwrap());
    let regex = regex::Regex::new(r"(?:API bound to 127.0.0.1:)(\d*)").unwrap();
    let mut lines = buf_reader.lines();
    let port: u16 = loop {
        let line = lines.next().unwrap()?;
        tracing::info!("Node {}: {}", id, line);
        if let Some(x) = regex.captures(&*line).and_then(|c| c.get(1).map(|x| x.as_str())) {
            break x.parse()?;
        }
    };
    tracing::info!("Started {} with port {}", id, port);
    let id = id.to_string();
    std::thread::spawn(move || {
        for line in lines {
            let line = line.unwrap();
            tracing::info!("Node {}: {}", id, line);
        }
    });
    let es = HttpClient::new_with_url(&*format!("http://localhost:{}/api/v2/events", port)).await?;

    Ok((node_handle, es))
}

#[tokio::test]
/// Simple test, spawning up two local nodes, emitting events into one, getting
/// them out via the other. Get output via:
/// RUST_LOG=two_nodes=DEBUG cargo test -- --color always --nocapture two_nodes
/// Note: This depends on working mdns on localhost.
async fn two_nodes() -> anyhow::Result<()> {
    init_tracing();
    // make sure actyx-linux binary is built and available
    for i in escargot::CargoBuild::new()
        .bin("actyx-linux")
        .manifest_path("../Cargo.toml")
        .exec()?
    {
        let i = i?;
        println!("escargot > {:?}", i);
    }

    let data_dir_1 = tempdir()?;
    let (mut node_1, es_1) = start_node("node_1", &data_dir_1).await?;
    let data_dir_2 = tempdir()?;
    let (mut node_2, es_2) = start_node("node_2", &data_dir_2).await?;

    const BATCH_SIZE: usize = 128;
    let mut data = Vec::with_capacity(BATCH_SIZE);
    for i in 0..BATCH_SIZE {
        data.push(i.to_string());
    }

    es_1.publish(PublishRequest {
        data: data
            .iter()
            .map(|i| Payload::compact(&i).unwrap())
            .map(|payload| PublishEvent {
                tags: tags!("my_tag"),
                payload,
            })
            .collect(),
    })
    .await?;
    let start = Instant::now();
    let offsets = loop {
        let offsets_observed_by_2 = es_2.offsets().await?;
        tracing::debug!("offsets_observed_by_2 {:?}", offsets_observed_by_2);
        // TODO: use actual stream id of node_1
        if offsets_observed_by_2.streams().count() >= 2 {
            break offsets_observed_by_2;
        }
        if start.elapsed() > Duration::from_millis(30000) {
            panic!("Didn't gossip in more than 30 s, giving up");
        }
        std::thread::sleep(Duration::from_millis(300));
    };
    let data_via_2: Vec<_> = es_2
        .query(QueryRequest {
            lower_bound: None,
            upper_bound: offsets.clone(),
            order: Order::Asc,
            query: "FROM 'my_tag'".parse().unwrap(),
        })
        .await?
        .map(|q| {
            let QueryResponse::Event(e) = q;
            assert_eq!(e.tags, tags!("my_tag"));
            let s: String = serde_json::from_value(e.payload.json_value()).unwrap();
            s
        })
        .collect()
        .await;

    tracing::debug!("data {} data_via_2 {}", data.len(), data_via_2.len());
    assert_eq!(data, data_via_2);

    node_1.kill().unwrap();
    node_2.kill().unwrap();
    Ok(())
}

pub fn init_tracing() {
    let fmt_layer = tracing_subscriber::fmt::Layer::default()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_ansi(false);
    let subscriber = tracing_subscriber::Registry::default()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(fmt_layer);
    tracing::subscriber::set_global_default(subscriber).expect("error setting global tracing subscriber");
}
