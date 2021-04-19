use std::{
    path::Path,
    time::{Duration, Instant},
};

use actyxos_sdk::{
    service::{EventService, Order, PublishEvent, PublishRequest, QueryRequest},
    tags, HttpClient, Payload,
};
use crossbeam::channel::Receiver;
use futures::StreamExt;
use logsvcd::LoggingSink;
use node::{ApplicationState, BindTo};
use tempfile::tempdir;
use util::formats::LogRequest;

fn get_eventservice_port(rx: &Receiver<LogRequest>) -> anyhow::Result<u16> {
    let start = Instant::now();
    let regex = regex::Regex::new(r"(?:API bound to 127.0.0.1:)(\d*)").unwrap();

    loop {
        let ev = rx.recv_deadline(start.checked_add(Duration::from_millis(5000)).unwrap())?;
        if let Some(x) = regex.captures(&*ev.message).and_then(|c| c.get(1).map(|x| x.as_str())) {
            return Ok(x.parse()?);
        }
    }
}

async fn start_node(
    path: impl AsRef<Path>,
    logging_rx: &Receiver<LogRequest>,
) -> anyhow::Result<(ApplicationState, HttpClient)> {
    let node = ApplicationState::spawn(
        path.as_ref().into(),
        node::Runtime::Linux,
        BindTo {
            api: "localhost:0".parse().unwrap(),
            ..BindTo::random()
        },
    )?;
    let port = get_eventservice_port(logging_rx)?;
    let es = HttpClient::new_with_url(&*format!("http://localhost:{}/api/v2/events", port)).await?;

    Ok((node, es))
}
#[tokio::test]
#[ignore]
async fn persistence_across_restarts() -> anyhow::Result<()> {
    // Install global subscriber before any app starts
    let (tx, logs_rx) = crossbeam::channel::unbounded();
    let _logging = LoggingSink::new(util::formats::LogSeverity::Debug, tx);
    let working_dir = tempdir()?;

    const BATCH_SIZE: usize = 2048;

    let mut data = Vec::with_capacity(BATCH_SIZE);
    for i in 0..BATCH_SIZE {
        data.push(i.to_string());
    }

    let (mut node, es) = start_node(&working_dir, &logs_rx).await?;

    let offsets_before = es.offsets().await?;
    es.publish(PublishRequest {
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
    let offsets_later = es.offsets().await?;

    let round_tripped: Vec<_> = es
        .query(QueryRequest {
            lower_bound: Some(offsets_before.clone()),
            upper_bound: offsets_later.clone(),
            order: Order::Asc,
            r#where: "'my_tag'".parse().unwrap(),
        })
        .await?
        .collect()
        .await;

    assert_eq!(round_tripped.len(), data.len());

    let offsets_before_shutdown = es.offsets().await?;
    // Tear down the node
    let rx = node.manager.rx_process.take().unwrap();
    drop(node);
    drop(es);
    let _ = rx.recv_timeout(Duration::from_millis(5000))?;

    // And start it up again
    let (_node, es) = start_node(&working_dir, &logs_rx).await?;

    let offsets_after_shutdown = es.offsets().await?;
    println!(
        "offsets_before {:?}, offsets_after {:?}",
        offsets_before_shutdown, offsets_after_shutdown
    );
    assert!(offsets_before_shutdown <= offsets_after_shutdown);

    let after_restart: Vec<_> = es
        .query(QueryRequest {
            lower_bound: Some(offsets_before),
            upper_bound: offsets_later,
            order: Order::Asc,
            r#where: "'my_tag'".parse().unwrap(),
        })
        .await?
        .collect()
        .await;

    assert_eq!(after_restart, round_tripped);

    Ok(())
}
