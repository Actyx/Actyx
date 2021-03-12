use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use crossbeam::channel;
use logsvcd::{GetLogRequest, Query, QueryMode, Storage, SyncLogService};
use std::time::Duration;
use util::formats::LogRequest;
use util::pinned_resource_sync::PinnedResourceSync;

fn mem_store(c: &mut Criterion) {
    const SAMPLE_SIZE: usize = 2048;
    const BATCH_SIZE: usize = 128;
    let mut logs_to_commit = Vec::with_capacity(SAMPLE_SIZE);
    for i in 0..SAMPLE_SIZE {
        logs_to_commit.push(LogRequest {
            additional_data: None,
            labels: None,
            log_name: "bench".to_string(),
            log_timestamp: None,
            message: i.to_string(),
            producer_name: "..".to_string(),
            producer_version: "..".to_string(),
            severity: util::formats::LogSeverity::Error,
        });
    }
    let (tx, rx) = channel::bounded(256);
    let (tx_conf, rx_conf) = channel::bounded(1);
    let (publish_tx, rx_pub) = channel::bounded(256);

    let storage = PinnedResourceSync::new(Storage::in_memory, "LogServiceWrapper::Storage");

    let mut log_service = SyncLogService::spawn_new(storage, rx, rx_pub, rx_conf).unwrap();
    let thread = std::thread::spawn(move || {
        log_service.run();
    });

    let (request, subscription_rx) = GetLogRequest::new(Query {
        follow: true,
        mode: QueryMode::All,
    });
    tx.send(request).unwrap();

    c.bench_function(
        &*format!(
            "logging roundtrip, sample size: {}, batch size: {}",
            SAMPLE_SIZE, BATCH_SIZE
        ),
        |b| {
            b.iter_batched(
                // prepare a clone of the input data
                || logs_to_commit.clone(),
                |mut input_logs| {
                    let mut received = 0usize;
                    for (cnt, log) in input_logs.drain(..).enumerate() {
                        (&publish_tx).send(log).unwrap();

                        // Drain the subscription channel
                        if ((cnt + 1) % BATCH_SIZE == 0) || ((cnt + 1) == SAMPLE_SIZE) {
                            while (cnt + 1) != received {
                                received += subscription_rx.recv().unwrap().len();
                            }
                        }
                    }
                    assert_eq!(received, SAMPLE_SIZE);
                },
                BatchSize::SmallInput,
            );
        },
    );

    // Thread will yield as soon as all senders have been dropped
    drop(tx_conf);
    drop(publish_tx);
    thread.join().unwrap();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).warm_up_time(Duration::from_secs(10)).measurement_time(Duration::from_secs(10));
    targets =  mem_store
}
criterion_main!(benches);
