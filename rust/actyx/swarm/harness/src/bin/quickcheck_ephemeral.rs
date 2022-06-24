#[cfg(target_os = "linux")]
fn main() {
    use actyx_sdk::{
        service::{EventMeta, EventResponse, EventService, QueryRequest, QueryResponse},
        service::{PublishEvent, PublishRequest},
        tags, OffsetMap, Payload,
    };
    use futures::{future, stream::FuturesUnordered, StreamExt};
    use libipld::{cbor::DagCborCodec, codec::Codec};
    use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
    use std::{
        fs::File,
        io::Read,
        time::{Duration, Instant},
    };
    use swarm_cli::{EphemeralEventsConfig, EphemeralEventsConfigWrapper, Event, RetainConfig};
    use swarm_harness::{api::Api, fully_meshed, run_netsim, setup_env, util::app_manifest, HarnessOpts};

    #[derive(Clone, Debug)]
    struct CountTest {
        retain_last_events: usize,
        events: usize,
    }

    impl Arbitrary for CountTest {
        fn arbitrary(g: &mut Gen) -> Self {
            let size = g.size();
            if size > 0 {
                let p = (0..size).collect::<Vec<_>>();
                let events = *g.choose(&p[..]).unwrap();
                let retain_last_events = *g.choose(&(0..events).collect::<Vec<_>>()[..]).unwrap_or(&0);
                Self {
                    retain_last_events,
                    events,
                }
            } else {
                Self {
                    events: 0,
                    retain_last_events: 0,
                }
            }
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let Self {
                events,
                retain_last_events,
            } = self.clone();
            Box::new(
                retain_last_events
                    .shrink()
                    .map(move |n| Self {
                        retain_last_events: n,
                        events,
                    })
                    .chain(events.shrink().map(move |n| Self {
                        events: n,
                        retain_last_events,
                    })),
            )
        }
    }

    #[derive(Clone, Debug)]
    struct SizeTest {
        retain_kbytes: usize,
        events: usize,
    }

    impl Arbitrary for SizeTest {
        fn arbitrary(g: &mut Gen) -> Self {
            let events = *g.choose(&(0..g.size()).collect::<Vec<_>>()[..]).unwrap();
            let retain_kbytes = *g.choose(&(0..events).collect::<Vec<_>>()[..]).unwrap_or(&0).max(&1);
            Self { retain_kbytes, events }
        }
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let Self { events, retain_kbytes } = self.clone();
            Box::new(
                retain_kbytes
                    .shrink()
                    .map(move |n| Self {
                        retain_kbytes: n,
                        events,
                    })
                    .chain(events.shrink().map(move |n| Self {
                        events: n,
                        retain_kbytes,
                    })),
            )
        }
    }

    fn make_events(n: usize) -> Vec<PublishEvent> {
        let mut buf = [0u8; 1021];
        // pruning is based on the compressed size of the values in a tree. So
        // make sure they don't compress well.
        let mut f = File::open("/dev/urandom").unwrap();
        (0..n)
            .map(|_| {
                f.read_exact(&mut buf[..]).unwrap();
                PublishEvent {
                    tags: tags!("a", "b"),
                    payload: Payload::compact(&buf.to_vec()).unwrap(),
                }
            })
            .collect()
    }

    fn run_test(
        retain_config: RetainConfig,
        events: Vec<PublishEvent>,
        min: usize,
        max: usize,
    ) -> quickcheck::TestResult {
        let opts = HarnessOpts {
            n_nodes: 5,
            n_bootstrap: 1,
            delay_ms: 0,
            enable_mdns: false,
            enable_fast_path: true,
            enable_slow_path: true,
            enable_root_map: true,
            enable_discovery: true,
            enable_metrics: true,
            enable_api: Some("0.0.0.0:30001".parse().unwrap()),
            ephemeral_events: Some(EphemeralEventsConfigWrapper(EphemeralEventsConfig::new(
                Duration::from_millis(100),
                maplit::btreemap! { 0.into() => retain_config },
            ))),
            // Force single event per leaf
            max_leaf_count: Some(1),
        };

        match run_netsim(opts, move |mut sim| async move {
            let api = Api::new(&mut sim, app_manifest())?;
            fully_meshed::<Event>(&mut sim, Duration::from_secs(60)).await?;

            let mut present = OffsetMap::empty();
            let machine = sim.machines().first().unwrap();
            api.run(machine.id(), move |client| async move {
                client.publish(PublishRequest { data: events }).await?;
                Ok(())
            })
            .await?;

            // Some time for pruning to happen
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Publish another event for other peers to ingest the new tree
            let (stream_0, max_offset) = api
                .run(machine.id(), move |client| async move {
                    let meta = client.publish(PublishRequest { data: make_events(1) }).await?;
                    let stream_0 = client.node_id().await.stream(0.into());
                    Ok((stream_0, meta.data.last().unwrap().offset))
                })
                .await?;
            present.update(stream_0, max_offset);

            tracing::debug!("offsets {:?}", present);
            let start = Instant::now();
            for m in sim.machines() {
                let id = m.id();
                loop {
                    let o = api.run(id, |c| async move { c.offsets().await }).await?.present;
                    if o >= present {
                        break;
                    }
                    anyhow::ensure!(start.elapsed() < Duration::from_secs(60));
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }

            let responses = tokio::time::timeout(
                Duration::from_secs(5),
                sim.machines()
                    .iter()
                    .map(|m| m.id())
                    .map(|id| {
                        api.run(id, |client| {
                            let request = QueryRequest {
                                lower_bound: None,
                                upper_bound: Some(present.clone()),
                                query: "FROM allEvents".parse().unwrap(),
                                order: actyx_sdk::service::Order::Asc,
                            };
                            async move {
                                let round_tripped = client
                                    .query(request)
                                    .await?
                                    .filter_map(|resp| async move {
                                        if let QueryResponse::Event(EventResponse {
                                            meta: EventMeta::Event { key, meta },
                                            payload,
                                        }) = resp
                                        {
                                            Some((key.stream, (meta.tags, payload)))
                                        } else {
                                            None
                                        }
                                    })
                                    .fold(0usize, |acc, _| future::ready(acc + 1))
                                    .await;

                                Result::<_, anyhow::Error>::Ok(round_tripped)
                            }
                        })
                    })
                    .collect::<FuturesUnordered<_>>()
                    .collect::<Vec<_>>(),
            )
            .await?
            .into_iter()
            .collect::<anyhow::Result<Vec<_>>>()?;
            tracing::debug!(
                "Expected min: {}, expected max: {}, received: {:?}",
                min,
                max,
                responses
            );
            anyhow::ensure!(
                responses
                    .iter()
                    // Depending on the point in time when pruning was
                    // triggered, the additional "trigger event" might be in
                    // or not
                    .all(|x| min <= *x && *x <= max + 1),
                "min: {}, max {}, x: {:?}",
                min,
                max,
                responses
            );

            Ok(())
        }) {
            Ok(()) => TestResult::passed(),
            Err(e) => {
                tracing::error!("Error from run: {:#?}", e);
                TestResult::error(format!("{:#?}", e))
            }
        }
    }

    fn ephemeral_pruning_size_based(input: SizeTest) -> quickcheck::TestResult {
        tracing::info!("TestInput {:?}", input);
        let retain_config = RetainConfig::Size(input.retain_kbytes as u64 * 1024);
        let events = make_events(input.events);
        let bytes_per_event_uncompressed = if let Some(f) = events.first().as_ref() {
            DagCborCodec.encode(&f.payload).unwrap().len()
        } else {
            1024
        };
        let (min, max) = {
            tracing::error!(
                "retain_kbytes {} bytes_per_event_uncompressed {}",
                input.retain_kbytes,
                bytes_per_event_uncompressed
            );
            // Compression ratio for the events generated above
            let bytes_per_event_compressed = 66 * bytes_per_event_uncompressed / 100;
            let target_events = (input.retain_kbytes * 1024 / bytes_per_event_compressed).min(input.events);
            // Fudge it
            (95 * target_events / 100, 105 * target_events / 100 + 1)
        };
        run_test(retain_config, events, min, max)
    }

    fn ephemeral_pruning_count_based(input: CountTest) -> quickcheck::TestResult {
        tracing::info!("TestInput {:?}", input);
        let retain_config = RetainConfig::Events(input.retain_last_events as u64);
        let events = make_events(input.events);
        let expected = input.retain_last_events.min(input.events);

        run_test(retain_config, events, expected, expected)
    }

    setup_env().unwrap();
    QuickCheck::new()
        .gen(Gen::new(1000))
        .tests(5)
        .quickcheck(ephemeral_pruning_count_based as fn(CountTest) -> TestResult);
    QuickCheck::new()
        .gen(Gen::new(1000))
        .tests(5)
        .quickcheck(ephemeral_pruning_size_based as fn(SizeTest) -> TestResult);
}

#[cfg(not(target_os = "linux"))]
fn main() {}
