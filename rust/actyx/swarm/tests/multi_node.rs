use actyxos_sdk::Payload;
use actyxos_sdk::{StreamNr, TagSet};
use anyhow::Result;
use ax_config::StoreConfig;
use banyan::query::AllQuery;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use swarm::BanyanStore;

fn setup_logger() {
    tracing_log::LogTracer::init().ok();
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).ok();
}

#[derive(Serialize, Deserialize)]
struct MyEvent {
    things_are_happening: Vec<String>,
}

#[tokio::test(flavor = "multi_thread")]
async fn banyan_multi_node() -> Result<()> {
    setup_logger();
    let config = StoreConfig::new("banyan-multi-node-test".to_string());
    let s1 = BanyanStore::from_axconfig(config.clone()).await?;
    let s2 = BanyanStore::from_axconfig(config.clone()).await?;

    // allow for stores to discover each other
    tokio::time::sleep(Duration::from_millis(1000)).await;

    let event = MyEvent {
        things_are_happening: vec!["hello world".to_string()],
    };

    let payload = Payload::compact(&event)?;
    let handle = tokio::spawn(async move {
        let mut stream = s2.stream_known_streams();
        let stream_nr = stream.next().await.unwrap();
        println!("known: {}", stream_nr);
        let mut stream = s2.stream_filtered_stream_ordered(AllQuery);
        let (i1, k1, e1) = stream.next().await.unwrap().unwrap();
        println!("{:?}", k1);
        assert_eq!(i1, 0);
        assert_eq!(e1, payload);
        let (i2, k2, e2) = stream.next().await.unwrap().unwrap();
        println!("{:?}", k2);
        assert_eq!(i2, 1);
        assert_eq!(e2, payload);
    });

    s1.append(StreamNr::from(1), vec![(TagSet::default(), Payload::compact(&event)?)])
        .await?
        .unwrap();
    s1.append(StreamNr::from(1), vec![(TagSet::default(), Payload::compact(&event)?)])
        .await?
        .unwrap();

    handle.await?;

    Ok(())
}
