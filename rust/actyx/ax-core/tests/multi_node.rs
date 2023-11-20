use actyx_sdk::{app_id, tags, AppId, Payload};
use ax_core::{
    swarm::BanyanStore,
    trees::query::{LamportQuery, TagExprQuery, TimeQuery},
};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct MyEvent {
    things_are_happening: Vec<String>,
}

#[tokio::test(flavor = "multi_thread")]
async fn banyan_multi_node() {
    ax_core::util::setup_logger();
    let s1 = BanyanStore::test("a").await.unwrap();
    let s2 = BanyanStore::test("b").await.unwrap();
    s1.ipfs()
        .clone()
        .add_address(s2.ipfs().local_peer_id(), s2.ipfs().listeners()[0].clone());

    fn app_id() -> AppId {
        app_id!("test")
    }

    let tags = tags!("event");
    let query = TagExprQuery::new(vec![tags.clone().into()], LamportQuery::all(), TimeQuery::all());

    let event = MyEvent {
        things_are_happening: vec!["hello world".to_string()],
    };

    let payload = Payload::compact(&event).unwrap();
    let handle = tokio::spawn(async move {
        let mut stream = s2.stream_known_streams();
        let stream_nr = stream.next().await.unwrap();
        tracing::info!("known: {}", stream_nr);
        let mut stream = s2.stream_filtered_stream_ordered(query);
        let (i1, k1, e1) = stream.next().await.unwrap().unwrap();
        tracing::info!("{:?}", k1);
        assert_eq!(i1, 4);
        assert_eq!(e1, payload);
        let (i2, k2, e2) = stream.next().await.unwrap().unwrap();
        tracing::info!("{:?}", k2);
        assert_eq!(i2, 5);
        assert_eq!(e2, payload);
    });

    s1.append(app_id(), vec![(tags.clone(), Payload::compact(&event).unwrap())])
        .await
        .unwrap();
    s1.append(app_id(), vec![(tags, Payload::compact(&event).unwrap())])
        .await
        .unwrap();

    handle.await.unwrap();
}
