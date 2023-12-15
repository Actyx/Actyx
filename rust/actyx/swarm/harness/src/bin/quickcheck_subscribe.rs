#[cfg(target_os = "linux")]
fn main() {
    use anyhow::Context;
    use async_std::{future::timeout, task::block_on};
    use ax_sdk::types::{
        service::{EventMeta, EventResponse, QueryResponse},
        tag, OffsetMap, TagSet,
    };
    use futures::{stream::FuturesUnordered, StreamExt};
    use quickcheck::{Gen, QuickCheck, TestResult};
    use std::{
        collections::BTreeMap,
        time::{Duration, Instant},
    };
    use swarm_cli::Event;
    use swarm_harness::{
        api::Api,
        fully_meshed, run_netsim, setup_env,
        util::{app_manifest, to_events},
        HarnessOpts,
    };

    const MAX_NODES: usize = 15;

    /// Publish arbitrary events on all nodes, subscribe to all of them on all nodes.
    fn publish_all_subscribe_all(tags_per_node: Vec<Vec<TagSet>>) -> quickcheck::TestResult {
        let n_nodes = tags_per_node.len().clamp(2, MAX_NODES);

        let opts = HarnessOpts {
            n_nodes,
            n_bootstrap: 1,
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
            event_routes: Default::default(),
        };

        let t = run_netsim(opts, move |mut sim| async move {
            let api = Api::new(&mut sim, app_manifest())?;
            fully_meshed::<Event>(&mut sim, Duration::from_secs(60)).await?;

            let mut present = OffsetMap::empty();
            let mut expected = BTreeMap::default();
            let mut publish = sim
                .machines()
                .iter()
                .zip(tags_per_node)
                .map(|(machine, tags)| {
                    api.run(machine.id(), move |client| async move {
                        let events = to_events(tags);
                        let e = events.clone(); // NOTE: Unsure how to do this better
                        let meta = client.execute(|ax| block_on(ax.publish().events(e))).await??;
                        let stream_0 = client.node_id().await.stream(0.into());
                        Result::<_, anyhow::Error>::Ok((stream_0, meta.data.last().map(|x| x.offset), events))
                    })
                })
                .collect::<FuturesUnordered<_>>();

            while let Some(x) = publish.next().await {
                let (stream_0, last_offset, evs) = x?;

                if let Some(offset) = last_offset {
                    present.update(stream_0, offset);
                    expected.insert(stream_0, evs);
                }
            }

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

            let mut queries = sim
                .machines()
                .iter()
                .map(|m| m.id())
                .map(|id| {
                    let upper_bound = present.clone();
                    api.run(id, move |client| async move {
                        let round_tripped = timeout(
                            Duration::from_secs(5),
                            client
                                .execute(|ax| block_on(ax.query("FROM allEvents").with_upper_bound(upper_bound)))
                                .await??
                                .filter_map(|resp| async move {
                                    match resp {
                                        QueryResponse::Event(EventResponse {
                                            meta: EventMeta::Event { key, meta },
                                            payload,
                                        }) if !meta.tags.contains(&tag!("files"))
                                            && !meta.tags.contains(&tag!("event_routing"))
                                            && !meta.tags.contains(&tag!("discovery")) =>
                                        {
                                            Some((key.stream, (meta.tags, payload)))
                                        }
                                        _ => None,
                                    }
                                })
                                .collect::<Vec<_>>(),
                        )
                        .await
                        .with_context(|| format!("query for {} timed out", id))?
                        .into_iter()
                        .fold(BTreeMap::default(), |mut acc, (stream, payload)| {
                            acc.entry(stream).or_insert_with(Vec::new).push(payload);
                            acc
                        });

                        Result::<_, anyhow::Error>::Ok(round_tripped)
                    })
                })
                .collect::<FuturesUnordered<_>>();
            while let Some(x) = queries.next().await {
                let round_tripped = x?;
                if expected != round_tripped {
                    anyhow::bail!("{:?} != {:?}", expected, round_tripped);
                }
            }

            Ok(())
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
        .gen(Gen::new(30))
        .tests(2)
        .quickcheck(publish_all_subscribe_all as fn(Vec<Vec<TagSet>>) -> TestResult)
}

#[cfg(not(target_os = "linux"))]
fn main() {}
