use ax_core::{
    node::{BindTo, Runtime},
    util::SocketAddrHelper,
};
use ax_sdk::{Ax, AxOpts};
use ax_types::{
    service::{Order, PublishEvent},
    tags, Payload,
};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use futures::StreamExt;
use std::time::Duration;
use tempfile::tempdir;

// Note: This doesn't concern itself with any internals (like flushing the send
// log etc). Just a simple and brute-force roundtrip test.
fn round_trip(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _x = ax_core::node::ApplicationState::spawn(
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
            || (data.clone(), Ax::new(AxOpts::default())),
            |(input, service)| async move {
                let service = service.await.unwrap();
                let offsets_before = service.offsets().await.unwrap();
                service
                    .publish()
                    .events(
                        input
                            .into_iter()
                            .map(|i| Payload::compact(&i).unwrap())
                            .map(|payload| PublishEvent {
                                tags: tags!("my_tag"),
                                payload,
                            }),
                    )
                    .await
                    .unwrap();
                let offsets_later = service.offsets().await.unwrap();
                let x: Vec<_> = service
                    .query("FROM 'my_tag'")
                    .with_order(Order::Asc)
                    .with_lower_bound(offsets_before.present)
                    .with_upper_bound(offsets_later.present)
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
