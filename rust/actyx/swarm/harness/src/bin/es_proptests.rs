use actyxos_sdk::Payload;
use actyxos_sdk::{
    app_id,
    service::{EventResponse, EventService, PublishEvent, PublishRequest, QueryRequest, QueryResponse},
    AppManifest, HttpClient, OffsetMap, TagSet, Url,
};
use futures::{stream::FuturesUnordered, StreamExt};
use netsim_embed::unshare_user;
use quickcheck::{QuickCheck, TestResult};
use std::collections::BTreeMap;
use swarm_harness::HarnessOpts;

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    unshare_user()?;
    let res = QuickCheck::new()
        .tests(10)
        .quicktest(test as fn(Vec<Vec<TagSet>>) -> anyhow::Result<TestResult>);
    if let Err(e) = res {
        panic!("{:?}", e);
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
            let app_manifest = AppManifest::new(
                app_id!("com.example.trial-mode"),
                "display name".into(),
                "0.1.0".into(),
                None,
            );
            let clients: Vec<HttpClient> = apis
                .into_iter()
                .map(|addr| Url::parse(&format!("http://{}", addr)).unwrap())
                .map(|url| HttpClient::new(url, app_manifest.clone()))
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
