#[cfg(target_os = "linux")]
fn main() {
    use std::{str::FromStr, time::Duration};

    use actyx_sdk::{
        language::TagExpr,
        service::{EventMeta, EventResponse, SubscribeResponse},
        tags, Offset, Payload, Url,
    };
    use async_std::task::block_on;
    use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
    use quickcheck::{empty_shrinker, Arbitrary, Gen};
    use quickcheck::{QuickCheck, TestResult};
    use swarm_cli::{Event, EventRoute};
    use swarm_harness::{api::ApiClient, m, run_netsim, setup_env, util::app_manifest, HarnessOpts};

    #[derive(Clone, Debug)]
    struct TestInput {
        concurrent_publishes: u8,
        publish_chunk_size: u8,
        publish_chunks_per_client: u8,
        concurrent_subscribes: u8,
    }
    impl Arbitrary for TestInput {
        fn arbitrary(g: &mut Gen) -> Self {
            let concurrent_publishes = (u8::arbitrary(g) >> 4).max(1);
            let publish_chunk_size = (u8::arbitrary(g) >> 2).max(1);
            let publish_chunks_per_client = (u8::arbitrary(g) >> 4).max(1);
            let concurrent_subscribes = u8::arbitrary(g).max(1);
            Self {
                concurrent_publishes,
                publish_chunk_size,
                publish_chunks_per_client,
                concurrent_subscribes,
            }
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            // Don't shrink
            empty_shrinker()
        }
    }

    fn stress_single_store(input: TestInput) -> quickcheck::TestResult {
        let TestInput {
            concurrent_publishes,
            publish_chunk_size,
            publish_chunks_per_client,
            concurrent_subscribes,
        } = input;

        let opts = HarnessOpts {
            n_nodes: 1,
            n_bootstrap: 0,
            delay_ms: 0,
            enable_mdns: false,
            enable_fast_path: true,
            enable_slow_path: true,
            enable_root_map: true,
            enable_discovery: true,
            enable_metrics: true,
            enable_api: Some("0.0.0.0:30001".parse().unwrap()),
            ephemeral_events: None,
            max_leaf_count: None,
            event_routes: vec![EventRoute::new(
                TagExpr::from_str("'my_test'").unwrap(),
                "test_stream".to_string(),
            )],
        };

        let t = run_netsim::<_, _, Event>(opts, move |mut sim| async move {
            tracing::info!(
                "running {}/{}/{}/{}",
                concurrent_publishes,
                publish_chunk_size,
                publish_chunks_per_client,
                concurrent_subscribes
            );
            let max_offset = Offset::try_from(
                (concurrent_publishes as u32 * publish_chunk_size as u32 * publish_chunks_per_client as u32) - 1,
            )
            .unwrap();

            let machine = &mut sim.machines_mut()[0];
            machine.send(swarm_cli::Command::ApiPort);
            let api_port = machine
                .select(|ev| m!(ev, Event::ApiPort(port) => *port))
                .await
                .ok_or_else(|| anyhow::anyhow!("machine died"))?
                .ok_or_else(|| anyhow::anyhow!("api endpoint not configured"))?;

            let origin = Url::parse(&format!("http://{}:{}", machine.addr(), api_port))?;
            let namespace = machine.namespace();

            let publish_clients = (0..concurrent_publishes)
                .map(|_| ApiClient::new(origin.clone(), app_manifest(), namespace))
                .collect::<Vec<_>>();

            let stream_1 = publish_clients[0].node_id().await.stream(1.into());

            let mut futs = publish_clients
                .iter()
                .enumerate()
                .map(|(i, client)| {
                    async move {
                        for c in 0..publish_chunks_per_client {
                            tracing::debug!(
                                "Client {}/{}: Chunk {}/{} (chunk size {})",
                                i + 1,
                                concurrent_publishes,
                                c + 1,
                                publish_chunks_per_client,
                                publish_chunk_size,
                            );
                            let _meta =
                                client
                                    .execute(move |ax| {
                                        block_on(ax.publish().events(
                                            (0..publish_chunk_size).map(|_| (tags!("my_test"), Payload::null())),
                                        ))
                                    })
                                    .await??;
                        }
                        tracing::info!("Client {}/{} done", i + 1, concurrent_publishes);
                        Result::<_, anyhow::Error>::Ok(())
                    }
                    .boxed()
                })
                .collect::<FuturesUnordered<_>>();

            for (id, client) in (0..concurrent_subscribes)
                .map(|_| ApiClient::new(origin.clone(), app_manifest(), namespace))
                .enumerate()
            {
                futs.push(
                    async move {
                        tracing::debug!("subscriber {} starting", id);
                        let req = client.execute(|ax| block_on(ax.subscribe("FROM 'my_test'"))).await?;
                        tracing::debug!(
                            "subscriber {} got {:?}",
                            id,
                            if let Err(e) = &req { Some(e) } else { None }
                        );
                        let mut req = req?;
                        tracing::info!("subscriber {} started", id);
                        while let Some(x) = tokio::time::timeout(Duration::from_secs(30), req.next()).await? {
                            if let SubscribeResponse::Event(EventResponse {
                                meta: EventMeta::Event { key, .. },
                                ..
                            }) = x
                            {
                                tracing::debug!("subscriber {} got offsets {}", id, key.offset);
                                if key.offset >= max_offset {
                                    tracing::info!("subscriber {} ended", id);
                                    return Ok(());
                                }
                            }
                        }
                        anyhow::bail!("Stream ended")
                    }
                    .boxed(),
                )
            }

            while let Some(res) = futs.next().await {
                if let Err(e) = res {
                    anyhow::bail!("{:#}", e);
                }
            }

            let present = publish_clients[0].offsets().await?;
            let actual = present.present.get(stream_1);
            if actual != Some(max_offset) {
                anyhow::bail!("{:?} != {:?}", actual, max_offset)
            } else {
                Ok(())
            }
        });
        match t {
            Ok(()) => TestResult::passed(),
            Err(e) => {
                tracing::error!("Error from run: {:#?}", e);
                TestResult::error(format!("{:#?}", e))
            }
        }
    }

    setup_env().unwrap();
    QuickCheck::new()
        .tests(2)
        .quickcheck(stress_single_store as fn(TestInput) -> TestResult)
}

#[cfg(not(target_os = "linux"))]
fn main() {}
