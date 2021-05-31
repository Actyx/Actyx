use actyxos_sdk::{
    service::{EventResponse, EventService, PublishEvent, PublishRequest, SubscribeRequest, SubscribeResponse},
    HttpClient, Offset, TagSet, Url,
};
use actyxos_sdk::{tags, Payload};
use async_std::task::block_on;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use netsim_embed::{unshare_user, Namespace};
use quickcheck::{QuickCheck, TestResult};
use std::{convert::TryFrom, num::NonZeroU8};
use swarm_cli::Event;
use swarm_harness::{api::ApiClient, util::app_manifest, HarnessOpts};
use util::pinned_resource::PinnedResource;

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    util::setup_logger();
    unshare_user()?;
    let res = QuickCheck::new()
        .tests(1)
        .quicktest(stress_single_store as fn(u8, u8, NonZeroU8, u8) -> TestResult);
    if let Err(e) = res {
        if e.is_failure() {
            panic!("{:?}", e);
        }
    }
    Ok(())
}
fn mk_client(origin: Url, namespace: Namespace) -> ApiClient {
    let pr = PinnedResource::new(move || {
        if let Err(e) = namespace.enter() {
            tracing::error!("cannot enter namespace {}: {}", namespace, e);
            panic!();
        }
        block_on(HttpClient::new(origin, app_manifest())).expect("cannot create")
    });
    ApiClient::new(pr)
}
fn stress_single_store(
    concurrent_publishes: u8,
    publish_chunk_size: u8,
    publish_chunks_per_client: NonZeroU8,
    concurrent_subscribes: u8,
) -> TestResult {
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
    };

    let t = swarm_harness::run_netsim::<_, _, Event>(opts, move |mut sim| async move {
        tracing::info!(
            "running {}/{}/{}/{}",
            concurrent_publishes,
            publish_chunk_size,
            publish_chunks_per_client,
            concurrent_subscribes
        );
        let maybe_max =
            (concurrent_publishes as u32 * publish_chunk_size as u32 * publish_chunks_per_client.get() as u32)
                .checked_sub(1)
                .map(|x| Offset::try_from(x).unwrap());

        let machine = &mut sim.machines_mut()[0];
        machine.send(swarm_cli::Command::ApiPort);
        let api_port = machine
            .select(|ev| swarm_harness::m!(ev, Event::ApiPort(port) => *port))
            .await
            .ok_or_else(|| anyhow::anyhow!("machine died"))?
            .ok_or_else(|| anyhow::anyhow!("api endpoint not configured"))?;

        let origin = Url::parse(&*format!("http://{}:{}", machine.addr(), api_port))?;
        let namespace = machine.namespace();

        let publish_clients = (0..concurrent_publishes)
            .map(|_| mk_client(origin.clone(), namespace))
            .collect::<Vec<_>>();

        let subscription_clients = (0..concurrent_subscribes)
            .map(|_| mk_client(origin.clone(), namespace))
            .collect::<Vec<_>>();

        let stream_0 = publish_clients[0].node_id().await?.node_id.stream(0.into());

        let mut futs = publish_clients
            .iter()
            .enumerate()
            .map(|(i, client)| {
                async move {
                    let tags = (0..publish_chunk_size).map(|_| tags!("my_test")).collect::<Vec<_>>();
                    let events = to_events(tags.clone());
                    for c in 0..publish_chunks_per_client.get() {
                        tracing::debug!(
                            "Client {}/{}: Chunk {}/{} (chunk size {})",
                            i + 1,
                            concurrent_publishes,
                            c + 1,
                            publish_chunks_per_client,
                            publish_chunk_size,
                        );
                        let _meta = client.publish(to_publish(events.clone())).await?;
                    }
                    Result::<_, anyhow::Error>::Ok(())
                }
                .boxed()
            })
            .collect::<FuturesUnordered<_>>();

        if let Some(max_offset) = maybe_max {
            let request = SubscribeRequest {
                offsets: None,
                query: "FROM 'my_test'".parse().unwrap(),
            };
            for client in subscription_clients {
                let request = request.clone();
                futs.push(
                    async move {
                        client
                            .subscribe(request)
                            .then(move |req| async move {
                                let mut req = req?;
                                while let Some(x) = req.next().await {
                                    let SubscribeResponse::Event(EventResponse { offset, .. }) = x;
                                    if offset >= max_offset {
                                        return Ok(());
                                    }
                                }
                                anyhow::bail!("Stream ended")
                            })
                            .await
                    }
                    .boxed(),
                )
            }
        }

        while let Some(res) = futs.next().await {
            if let Err(e) = res {
                anyhow::bail!("{:#}", e);
            }
        }

        let present = publish_clients[0].offsets().await?;
        let actual = present.present.get(stream_0);
        if actual != maybe_max {
            anyhow::bail!("{:?} != {:?}", actual, maybe_max)
        } else {
            Ok(())
        }
    });
    match t {
        Ok(()) => TestResult::passed(),
        Err(e) => TestResult::error(format!("{:#?}", e)),
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
