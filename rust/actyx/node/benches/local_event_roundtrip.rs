use actyx_sdk::{
    app_id,
    service::{EventService, Order, PublishEvent, PublishRequest, QueryRequest},
    tags, AppManifest, HttpClient, Payload,
};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use futures::StreamExt;
use node::{BindTo, Runtime};
use std::time::Duration;
use tempfile::tempdir;
use url::Url;
use util::SocketAddrHelper;

async fn mk_http_client() -> anyhow::Result<HttpClient> {
    let app_manifest = AppManifest::new(
        app_id!("com.example.trial-mode"),
        "display name".into(),
        "0.1.0".into(),
        None,
    );
    let url = Url::parse("http://localhost:4454").unwrap();
    HttpClient::new(url, app_manifest).await
}

// Note: This doesn't concern itself with any internals (like flushing the send
// log etc). Just a simple and brute-force roundtrip test.
fn round_trip(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _x = node::ApplicationState::spawn(
        dir.path().into(),
        Runtime::Linux,
        BindTo {
            api: SocketAddrHelper::unspecified(4454).unwrap(),
            ..Default::default()
        },
        false,
        false,
    )
    .unwrap();

    // Some time for startup
    std::thread::sleep(Duration::from_millis(500));
    const BATCH_SIZE: usize = 2048;

    let mut data = Vec::with_capacity(BATCH_SIZE);
    for i in 0..BATCH_SIZE {
        data.push(i.to_string());
    }
    c.bench_function("id", |b| {
        b.to_async(&rt).iter_batched(
            || (data.clone(), mk_http_client()),
            |(input, service)| async move {
                let service = service.await.unwrap();
                let offsets_before = service.offsets().await.unwrap();
                service
                    .publish(PublishRequest {
                        data: input
                            .into_iter()
                            .map(|i| Payload::compact(&i).unwrap())
                            .map(|payload| PublishEvent {
                                tags: tags!("my_tag"),
                                payload,
                            })
                            .collect(),
                    })
                    .await
                    .unwrap();
                let offsets_later = service.offsets().await.unwrap();
                let x: Vec<_> = service
                    .query(QueryRequest {
                        lower_bound: Some(offsets_before.present),
                        upper_bound: Some(offsets_later.present),
                        order: Order::Asc,
                        query: "FROM 'my_tag'".parse().unwrap(),
                    })
                    .await
                    .unwrap()
                    .collect()
                    .await;
                assert_eq!(x.len(), BATCH_SIZE);
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).warm_up_time(Duration::from_secs(10)).measurement_time(Duration::from_secs(10));
    targets =  round_trip
}
criterion_main!(benches);
