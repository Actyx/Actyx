use actyxos_sdk::{
    language::{Query, TagAtom, TagExpr},
    service::{
        EventResponse, EventService, PublishEvent, PublishRequest, QueryRequest, QueryResponse, SubscribeRequest,
        SubscribeResponse,
    },
    Offset, OffsetMap, Tag, TagSet,
};
use actyxos_sdk::{tags, Payload};
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use netsim_embed::unshare_user;
use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
use std::{collections::BTreeMap, convert::TryFrom, num::NonZeroU8, str::FromStr, time::Duration};
use swarm_harness::util::{await_stream_offsets, mk_client, mk_clients, run_quickcheck};

const MAX_NODES: usize = 20;
#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    util::setup_logger();
    unshare_user()?;

    let res = QuickCheck::new()
        .tests(10)
        .quicktest(interleaved as fn(TestInput) -> anyhow::Result<TestResult>);
    if let Err(e) = res {
        panic!("{:?}", e);
    }

    let res = QuickCheck::new()
        .tests(10)
        .quicktest(publish_all_subscribe_all as fn(Vec<Vec<TagSet>>) -> anyhow::Result<TestResult>);
    if let Err(e) = res {
        panic!("{:?}", e);
    }

    let res = QuickCheck::new()
        .tests(10)
        .quicktest(stress_single_store as fn(u8, u8, NonZeroU8, u8) -> anyhow::Result<TestResult>);
    if let Err(e) = res {
        if e.is_failure() {
            panic!("{:?}", e);
        }
    }
    Ok(())
}

fn publish_all_subscribe_all(tags_per_node: Vec<Vec<TagSet>>) -> anyhow::Result<TestResult> {
    if tags_per_node.len() < 2 {
        return Ok(TestResult::discard());
    }
    let n_nodes = tags_per_node.len().max(2).min(MAX_NODES);

    run_quickcheck(n_nodes, move |apis| async move {
        let clients = mk_clients(apis).await?;

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
}

#[derive(Clone, Debug)]
enum TestCommand {
    Subscribe {
        tags: TagSet,
        node: usize, // index into nodes array
    },
    Publish {
        node: usize, // index into nodes array
        tags: Vec<TagSet>,
    },
}

#[derive(Clone, Debug)]
struct TestInput {
    n_nodes: usize,
    commands: Vec<TestCommand>,
    cnt_per_tagset: BTreeMap<TagSet, usize>,
}
fn to_query(tags: TagSet) -> Query {
    let from = tags
        .iter()
        .map(TagAtom::Tag)
        .map(TagExpr::Atom)
        .reduce(|a, b| a.and(b))
        .unwrap_or(TagExpr::Atom(TagAtom::AllEvents));
    Query { from, ops: vec![] }
}
fn cnt_per_tag(cmds: &[TestCommand]) -> BTreeMap<TagSet, usize> {
    let mut map: BTreeMap<TagSet, usize> = Default::default();
    for c in cmds {
        if let TestCommand::Publish { tags, .. } = c {
            for t in tags {
                *map.entry(t.clone()).or_default() += 1;
            }
        }
    }
    map
}
impl Arbitrary for TestInput {
    fn arbitrary(g: &mut Gen) -> Self {
        let n = (Vec::<bool>::arbitrary(g).len() % MAX_NODES).max(1); // 0 < nodes <= MAX_NODES
        let nodes: Vec<usize> = (0..n).into_iter().enumerate().map(|(i, _)| i).collect();
        // fancy tagset don't really matter here
        let possible_tagsets = Vec::<Vec<bool>>::arbitrary(g)
            .into_iter()
            .enumerate()
            .map(|(idx, v)| {
                v.into_iter()
                    .enumerate()
                    .map(|(idx2, _)| Tag::from_str(&*format!("{}-{}", idx, idx2)).unwrap())
                    .collect::<TagSet>()
            })
            .collect::<Vec<_>>();
        let commands = Vec::<bool>::arbitrary(g)
            .into_iter()
            .map(|_| {
                match g.choose(&[0, 1, 2]).unwrap() {
                    1 => TestCommand::Subscribe {
                        tags: g.choose(&possible_tagsets[..]).cloned().unwrap_or_default(),
                        node: *g.choose(&nodes[..]).unwrap(),
                    },
                    _ => {
                        let tags = possible_tagsets
                            .iter()
                            .filter(|_| bool::arbitrary(g))
                            .cloned()
                            .collect();
                        TestCommand::Publish {
                            tags,
                            node: *g.choose(&nodes[..]).unwrap(), // stream: possible_streams,
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        Self {
            cnt_per_tagset: cnt_per_tag(&commands),
            commands,
            n_nodes: nodes.len(),
        }
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(TestShrinker::new(self.clone()))
    }
}
enum ShrinkState {
    ShrinkNodes,
    ShrinkCommands,
}
struct TestShrinker {
    seed: TestInput,
    last: TestInput,
    state: ShrinkState,
}
impl TestShrinker {
    fn new(seed: TestInput) -> Self {
        Self {
            last: seed.clone(),
            seed,
            state: ShrinkState::ShrinkNodes,
        }
    }
}
impl Iterator for TestShrinker {
    type Item = TestInput;
    fn next(&mut self) -> Option<Self::Item> {
        tracing::info!("Shrinking from {}/{}", self.seed.n_nodes, self.seed.commands.len());
        loop {
            match &mut self.state {
                ShrinkState::ShrinkNodes => {
                    if self.last.n_nodes > 1 {
                        // Try with less nodes
                        self.last.n_nodes /= 2;
                        break Some(self.last.clone());
                    } else {
                        // less nodes didn't work :-(
                        self.last = self.seed.clone();
                        self.state = ShrinkState::ShrinkCommands;
                    }
                }
                ShrinkState::ShrinkCommands => {
                    if self.last.commands.len() > 2 {
                        let len = self.last.commands.len();
                        self.last.commands.drain(len - 2..len);
                        self.last.cnt_per_tagset = cnt_per_tag(&self.last.commands);
                        break Some(self.last.clone());
                    } else {
                        // less commands didn't work :-(
                        // give up
                        break None;
                    }
                }
            }
        }
    }
}

fn interleaved(input: TestInput) -> anyhow::Result<TestResult> {
    let TestInput {
        commands,
        n_nodes,
        cnt_per_tagset,
    } = input;
    tracing::info!("{} nodes with {} commands", n_nodes, commands.len(),);

    run_quickcheck(n_nodes, move |apis| async move {
        std::thread::sleep(Duration::from_millis(500));
        let clients = mk_clients(apis).await?;
        let mut futs = commands
            .into_iter()
            .enumerate()
            .map(move |(cmd_id, cmd)| match cmd {
                TestCommand::Publish { tags, node } => {
                    let events = to_events(tags);
                    let node = node % n_nodes;
                    let client = clients[node].clone();
                    tracing::debug!("Cmd {} / Node {}: Publishing {} events", cmd_id, node, events.len());
                    async move {
                        client.publish(to_publish(events)).await?;
                        Result::<_, anyhow::Error>::Ok(())
                    }
                    .boxed()
                }
                TestCommand::Subscribe { node, tags, .. } => {
                    let expected_cnt = *cnt_per_tagset.get(&tags).unwrap_or(&0);
                    let query = to_query(tags);
                    // let expected_cnt = query.from
                    let request = SubscribeRequest { offsets: None, query };
                    let node = node % n_nodes;
                    let client = clients[node].clone();
                    tracing::debug!(
                        "Cmd {} / Node {}: subscribing, expecting {} events",
                        cmd_id,
                        node,
                        expected_cnt
                    );
                    async move {
                        let mut req = client.subscribe(request).await?;
                        let mut actual = 0;
                        if expected_cnt > 0 {
                            while tokio::time::timeout(Duration::from_millis(10_000), req.next())
                                .await?
                                .is_some()
                            {
                                actual += 1;
                                tracing::debug!("Cmd {} / Node {}: {}/{}", cmd_id, node, actual, expected_cnt,);
                                if actual >= expected_cnt {
                                    tracing::debug!("Cmd {} / Node {}: Done", cmd_id, node);
                                    break;
                                }
                            }
                        }
                        Result::<_, anyhow::Error>::Ok(())
                    }
                    .boxed()
                }
            })
            .collect::<FuturesUnordered<_>>();

        while let Some(res) = futs.next().await {
            if let Err(e) = res {
                return Ok(TestResult::error(format!("{:#}", e)));
            }
        }
        Ok(TestResult::passed())
    })
}

fn stress_single_store(
    clients: u8,
    chunk_size: u8,
    chunks_per_client: NonZeroU8,
    concurrent_requests: u8,
) -> anyhow::Result<TestResult> {
    run_quickcheck(1, move |apis| async move {
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
        let client = mk_client(apis[0]).await?;
        let mut futs = (0..clients)
            .map(|i| {
                let client = client.clone();
                async move {
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
                }
                .boxed()
            })
            .collect::<FuturesUnordered<_>>();

        let stream_0 = client.node_id().await?.node_id.stream(0.into());
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
