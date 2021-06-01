use actyxos_sdk::Payload;
use actyxos_sdk::{
    service::{EventResponse, EventService, PublishEvent, PublishRequest, QueryRequest, QueryResponse},
    OffsetMap, TagSet,
};
use futures::{stream::FuturesUnordered, StreamExt};
use quickcheck::{Gen, QuickCheck, TestResult};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use swarm_cli::Event;
use swarm_harness::api::Api;
use swarm_harness::util::app_manifest;
use swarm_harness::{fully_meshed, HarnessOpts};

const MAX_NODES: usize = 15;
#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    swarm_harness::setup_env()?;
    let res = QuickCheck::new()
        .gen(Gen::new(30))
        .tests(2)
        .quicktest(publish_all_subscribe_all as fn(Vec<Vec<TagSet>>) -> TestResult);
    if let Err(e) = res {
        panic!("{:?}", e);
    }

    Ok(())
}

/// Publish arbitrary events on all nodes, subscribe to all of them on all nodes.
fn publish_all_subscribe_all(tags_per_node: Vec<Vec<TagSet>>) -> TestResult {
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
    };

    let t = swarm_harness::run_netsim(opts, move |mut sim| async move {
        let api = Api::new(&mut sim, app_manifest())?;
        fully_meshed::<Event>(&mut sim, Duration::from_secs(60)).await;

        let mut present = OffsetMap::empty();
        let mut expected = BTreeMap::default();
        let mut publish = sim
            .machines()
            .iter()
            .zip(tags_per_node)
            .map(|(machine, tags)| {
                api.run(machine.id(), move |client| async move {
                    let events = to_events(tags);
                    let meta = client.publish(to_publish(events.clone())).await?;
                    let stream_0 = client.node_id().await?.node_id.stream(0.into());
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
                api.run(id, |client| {
                    let request = QueryRequest {
                        lower_bound: None,
                        upper_bound: present.clone(),
                        query: "FROM allEvents".parse().unwrap(),
                        order: actyxos_sdk::service::Order::Asc,
                    };
                    async move {
                        let round_tripped = client
                            .query(request)
                            .await?
                            .map(|x| {
                                let QueryResponse::Event(EventResponse {
                                    tags, payload, stream, ..
                                }) = x;
                                (stream, (tags, payload))
                            })
                            .collect::<Vec<_>>()
                            .await
                            .into_iter()
                            .fold(BTreeMap::default(), |mut acc, (stream, payload)| {
                                acc.entry(stream).or_insert_with(Vec::new).push(payload);
                                acc
                            });

                        Result::<_, anyhow::Error>::Ok(round_tripped)
                    }
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

fn to_events(tags: Vec<TagSet>) -> Vec<(TagSet, Payload)> {
    tags.into_iter().map(|t| (t, Payload::empty())).collect()
}
fn to_publish(events: Vec<(TagSet, Payload)>) -> PublishRequest {
    PublishRequest {
        data: events
            .into_iter()
            .map(|(tags, payload)| PublishEvent { tags, payload })
            .collect(),
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {}
