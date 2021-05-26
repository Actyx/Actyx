use actyxos_sdk::{
    app_id,
    service::{
        EventResponse, EventService, PublishEvent, PublishRequest, QueryRequest, QueryResponse, SubscribeRequest,
        SubscribeResponse,
    },
    AppManifest, HttpClient, Offset, OffsetMap, TagSet, Url,
};
use actyxos_sdk::{tags, Payload};
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use netsim_embed::unshare_user;
use quickcheck::{QuickCheck, TestResult};
use std::{collections::BTreeMap, convert::TryFrom, num::NonZeroU8, time::Duration};
use swarm_harness::HarnessOpts;

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    unshare_user()?;
    if true {
        let res = QuickCheck::new()
            .tests(10)
            .quicktest(test as fn(Vec<Vec<TagSet>>) -> anyhow::Result<TestResult>);
        if let Err(e) = res {
            panic!("{:?}", e);
        }
    }

    if false {
        let res = QuickCheck::new()
            .tests(10)
            // .gen(Gen::new(100_000))
            .quicktest(hammer_store as fn(u8, u8, NonZeroU8, u8) -> anyhow::Result<TestResult>);
        if let Err(e) = res {
            if e.is_failure() {
                panic!("{:?}", e);
            }
        }
    }
    // TODO: Add test case to query subset of persisted events
    Ok(())
}
async fn await_stream_offsets(stores: &[HttpClient], target_offsets: &OffsetMap) -> anyhow::Result<()> {
    for store in stores {
        loop {
            let o = store.offsets().await?.present;
            if o >= *target_offsets {
                break;
            }
        }
    }
    Ok(())
}
fn test(tags_per_node: Vec<Vec<TagSet>>) -> anyhow::Result<TestResult> {
    if tags_per_node.len() < 2 {
        return Ok(TestResult::discard());
    }
    // TODO increase?
    let n_nodes = tags_per_node.len().max(2).min(16);
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
    std::thread::spawn(|| {
        swarm_harness::run_netsim_quickcheck(opts, |apis| async move {
            let clients: Vec<HttpClient> = apis
                .into_iter()
                .map(|addr| Url::parse(&format!("http://{}", addr)).unwrap())
                .map(|url| HttpClient::new(url, app_manifest()))
                .collect::<FuturesUnordered<_>>()
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<anyhow::Result<_>>()?;

            let mut present = OffsetMap::empty();
            let mut expected = BTreeMap::default();
            let mut publish = clients
                .iter()
                .zip(tags_per_node)
                .map(|(client, tags)| async move {
                    let events = to_events(tags);
                    let meta = client.publish(to_publish(events.clone())).await?;
                    let stream_0 = client.node_id().await?.node_id.stream(0.into());
                    Result::<_, anyhow::Error>::Ok((stream_0, meta.data.last().map(|x| x.offset), events))
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
            await_stream_offsets(&clients[..], &present).await?;
            let mut queries = clients
                .iter()
                .map(|client| {
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
                .collect::<FuturesUnordered<_>>();
            while let Some(x) = queries.next().await {
                let round_tripped = x?;
                if expected != round_tripped {
                    return Ok(TestResult::error(format!("{:?} != {:?}", expected, round_tripped)));
                }
            }

            Ok(TestResult::passed())
        })
    })
    .join()
    .unwrap()
}
fn hammer_store(
    clients: u8,
    chunk_size: u8,
    chunks_per_client: NonZeroU8,
    concurrent_requests: u8,
) -> anyhow::Result<TestResult> {
    let opts = HarnessOpts {
        n_nodes: 1,
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
    let r = std::thread::spawn(move || {
        swarm_harness::run_netsim_quickcheck(opts, |apis| async move {
            std::thread::sleep(Duration::from_millis(500));
            tracing::info!(
                "running {}/{}/{}/{}",
                clients,
                chunk_size,
                chunks_per_client,
                concurrent_requests
            );
            let maybe_max = (clients as u32 * chunk_size as u32 * chunks_per_client.get() as u32)
                .checked_sub(1)
                .map(|x| Offset::try_from(x).unwrap());
            let client = HttpClient::new(Url::parse(&format!("http://{}", apis[0]))?, app_manifest()).await?;
            let mut futs = (0..clients)
                .map(|i| {
                    HttpClient::new(Url::parse(&format!("http://{}", apis[0])).unwrap(), app_manifest())
                        .then(move |client| async move {
                            let client = client?;
                            let tags = (0..chunk_size).map(|_| tags!("my_test")).collect::<Vec<_>>();
                            let events = to_events(tags.clone());
                            for c in 0..chunks_per_client.get() {
                                tracing::debug!(
                                    "Client {}/{}: Chunk {}/{} (chunk size {})",
                                    i + 1,
                                    clients,
                                    c + 1,
                                    chunks_per_client,
                                    chunk_size,
                                );
                                // Slow is ok, but stalled is not
                                let _meta = tokio::time::timeout(
                                    Duration::from_millis(chunk_size as u64 * 10),
                                    client.publish(to_publish(events.clone())),
                                )
                                .await??;
                            }
                            Result::<_, anyhow::Error>::Ok(())
                        })
                        .boxed()
                })
                .collect::<FuturesUnordered<_>>();

            let stream_0 = client.node_id().await?.node_id.stream(0.into());
            println!("maybe_max {:?}", maybe_max);
            if let Some(max_offset) = maybe_max {
                let request = SubscribeRequest {
                    offsets: None,
                    query: "FROM 'my_test'".parse().unwrap(),
                };
                for _ in 0..concurrent_requests {
                    let request = request.clone();
                    futs.push(
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
                            .boxed(),
                    );
                }
            }

            while let Some(res) = futs.next().await {
                if let Err(e) = res {
                    println!("err {:?}", e);
                    return Ok(TestResult::error(format!("{:#}", e)));
                }
            }

            let present = client.offsets().await?;
            let actual = present.present.get(stream_0);
            if actual != maybe_max {
                Ok(TestResult::error(format!("{:?} != {:?}", actual, maybe_max)))
            } else {
                Ok(TestResult::passed())
            }
        })
    })
    .join()
    .unwrap();
    println!("r from thread {:?}", r);
    r
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
fn app_manifest() -> AppManifest {
    AppManifest::new(
        app_id!("com.example.trial-mode"),
        "display name".into(),
        "0.1.0".into(),
        None,
    )
}
