use actyx_sdk::{tags, Payload, StreamNr};
use anyhow::Result;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use swarm::BanyanStore;
use trees::query::TagsQuery;

#[derive(Serialize, Deserialize)]
struct MyEvent {
    things_are_happening: Vec<String>,
}

#[tokio::test(flavor = "multi_thread")]
async fn banyan_multi_node() -> Result<()> {
    util::setup_logger();
    let s1 = BanyanStore::test("a").await?;
    let s2 = BanyanStore::test("b").await?;
    s1.ipfs()
        .add_address(&s2.ipfs().local_peer_id(), s2.ipfs().listeners()[0].clone());

    let tags = tags!("event");
    let query = TagsQuery::new(vec![tags.clone().into()]);

    let event = MyEvent {
        things_are_happening: vec!["hello world".to_string()],
    };

    let payload = Payload::compact(&event)?;
    let handle = tokio::spawn(async move {
        let mut stream = s2.stream_known_streams();
        let stream_nr = stream.next().await.unwrap();
        tracing::info!("known: {}", stream_nr);
        let mut stream = s2.stream_filtered_stream_ordered(query);
        let (i1, k1, e1) = stream.next().await.unwrap().unwrap();
        tracing::info!("{:?}", k1);
        assert_eq!(i1, 0);
        assert_eq!(e1, payload);
        let (i2, k2, e2) = stream.next().await.unwrap().unwrap();
        tracing::info!("{:?}", k2);
        assert_eq!(i2, 1);
        assert_eq!(e2, payload);
    });

    s1.append(StreamNr::from(11), vec![(tags.clone(), Payload::compact(&event)?)])
        .await?;
    s1.append(StreamNr::from(11), vec![(tags, Payload::compact(&event)?)])
        .await?;

    handle.await?;

    Ok(())
}
