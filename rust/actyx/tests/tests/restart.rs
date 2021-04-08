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
use node::{ApplicationState, BindTo};
use tempfile::tempdir;
use tokio::time::sleep;
use util::formats::LogEvent;

fn get_eventservice_port(rx: Receiver<Vec<LogEvent>>) -> anyhow::Result<u16> {
    let start = Instant::now();
    let regex = regex::Regex::new(r"(?:API bound to 127.0.0.1:)(\d*)").unwrap();

    loop {
        let chunk = rx.recv_deadline(start.checked_add(Duration::from_millis(5000)).unwrap())?;
        for ev in chunk {
            if let Some(x) = regex.captures(&*ev.message).and_then(|c| c.get(1).map(|x| x.as_str())) {
                return Ok(x.parse()?);
            }
        }
    }
}

async fn start_node(
    path: impl AsRef<Path>,
    with_port: Option<u16>,
) -> anyhow::Result<(ApplicationState, HttpClient, u16)> {
    let node = ApplicationState::spawn(
        path.as_ref().into(),
        node::Runtime::Linux,
        BindTo {
            api: format!("localhost:{}", with_port.unwrap_or(0)).parse().unwrap(),
            ..BindTo::random()
        },
    )?;
    let port = if let Some(p) = with_port {
        p
    } else {
        get_eventservice_port(node.logs_tail()?)?
    };
    let es = HttpClient::new_with_url(&*format!("http://localhost:{}/api/v2/events", port)).await?;
    // Some time for everything to be started up. The main issue here is on the
    // second start, our connection to the logging systems is broken, thus we
    // can't wait for the proper message to appear. FIXME
    sleep(Duration::from_millis(1000)).await;

    Ok((node, es, port))
}
#[tokio::test]
async fn persistence_across_restarts() -> anyhow::Result<()> {
    let working_dir = tempdir()?;

    const BATCH_SIZE: usize = 2048;

    let mut data = Vec::with_capacity(BATCH_SIZE);
    for i in 0..BATCH_SIZE {
        data.push(i.to_string());
    }

    let (mut node, es, es_port) = start_node(&working_dir, None).await?;

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
            query: "FROM 'my_tag'".parse().unwrap(),
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

    // And start it up again; reusing the same port
    let (_node, es, _) = start_node(&working_dir, Some(es_port)).await?;

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
            query: "FROM 'my_tag'".parse().unwrap(),
        })
        .await?
        .collect()
        .await;

    assert_eq!(after_restart, round_tripped);

    Ok(())
}
