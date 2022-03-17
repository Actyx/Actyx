#[cfg(target_os = "linux")]
fn main() {
    use std::{collections::BTreeMap, str::FromStr, time::Duration};

    use actyx_sdk::{
        language::{Query, TagAtom, TagExpr},
        service::{EventService, SubscribeRequest},
        Tag, TagSet,
    };
    use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
    use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
    use swarm_cli::Event;
    use swarm_harness::{
        api::ApiClient,
        fully_meshed, run_netsim, setup_env,
        util::{app_manifest, to_events, to_publish},
        HarnessOpts,
    };

    const MAX_NODES: usize = 15;

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
    pub struct TestInput {
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
        Query {
            features: vec![],
            from,
            ops: vec![],
        }
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
            let nodes: Vec<usize> = (0..n).collect();
            // fancy tagset don't really matter here
            let possible_tagsets = Vec::<Vec<bool>>::arbitrary(g)
                .into_iter()
                .map(|v| {
                    v.into_iter()
                        .enumerate()
                        .map(|(idx, _)| Tag::from_str(&*format!("{}", idx)).unwrap())
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
    #[derive(Debug, PartialEq)]
    enum ShrinkState {
        ShrinkNodes,
        ShrinkCommands,
        Exhausted,
    }
    struct TestShrinker {
        seed: TestInput,
        last: TestInput,
        state: ShrinkState,
        pending: Box<dyn Iterator<Item = TestInput>>,
    }
    impl TestShrinker {
        fn new(seed: TestInput) -> Self {
            Self {
                last: seed.clone(),
                seed,
                state: ShrinkState::ShrinkNodes,
                pending: Box::new(std::iter::empty()),
            }
        }
    }
    impl Iterator for TestShrinker {
        type Item = TestInput;
        fn next(&mut self) -> Option<Self::Item> {
            if self.state == ShrinkState::Exhausted {
                return self.pending.next();
            }
            loop {
                tracing::info!(
                    "Shrinking from {}/{}: {:?}",
                    self.seed.n_nodes,
                    self.seed.commands.len(),
                    self.state
                );
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
                            // try reducing tags and then give up
                            self.state = ShrinkState::Exhausted;

                            self.last = self.seed.clone();
                            let cloned = self.last.clone();
                            let x = Box::new(self.last.clone().commands.into_iter().enumerate().flat_map(
                                move |(idx, x)| {
                                    let cloned = cloned.clone();
                                    let y: Box<dyn Iterator<Item = TestInput>> = match x {
                                        TestCommand::Publish { node, tags } => {
                                            Box::new(tags.shrink().map(move |tags| {
                                                let cmd = TestCommand::Publish { node, tags };
                                                let mut c = cloned.clone();
                                                c.commands[idx] = cmd;
                                                c.cnt_per_tagset = cnt_per_tag(&c.commands);
                                                c
                                            }))
                                        }
                                        TestCommand::Subscribe { node, tags } => {
                                            Box::new(tags.shrink().map(move |tags| {
                                                let cmd = TestCommand::Subscribe { node, tags };
                                                let mut c = cloned.clone();
                                                c.commands[idx] = cmd;
                                                c.cnt_per_tagset = cnt_per_tag(&c.commands);
                                                c
                                            }))
                                        }
                                    };
                                    y
                                },
                            ));
                            tracing::debug!("Shrunk tags");
                            self.pending = x;
                            return self.pending.next();
                        }
                    }
                    ShrinkState::Exhausted => unreachable!(),
                }
            }
        }
    }

    fn interleaved(input: TestInput) -> TestResult {
        let TestInput {
            commands,
            n_nodes,
            cnt_per_tagset,
        } = input;
        tracing::info!("{} nodes with {} commands", n_nodes, commands.len(),);
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
        };

        let t = run_netsim::<_, _, Event>(opts, move |mut sim| async move {
            fully_meshed::<Event>(&mut sim, Duration::from_secs(60)).await?;
            let machines = sim.machines().iter().map(|m| m.id()).collect::<Vec<_>>();
            assert_eq!(machines.len(), n_nodes);
            let mut futs = commands
                .into_iter()
                .enumerate()
                .map(|(cmd_id, cmd)| match cmd {
                    TestCommand::Publish { tags, node } => {
                        let id = machines[node % n_nodes];
                        let client = ApiClient::from_machine(sim.machine(id), app_manifest(), None).unwrap();
                        let events = to_events(tags);
                        tracing::debug!("Cmd {} / Node {}: Publishing {} events", cmd_id, node, events.len());
                        async move {
                            client.publish(to_publish(events.clone())).await?;
                            Result::<_, anyhow::Error>::Ok(())
                        }
                        .boxed()
                    }
                    TestCommand::Subscribe { node, tags, .. } => {
                        let expected_cnt = *cnt_per_tagset.get(&tags).unwrap_or(&0);
                        tracing::debug!(
                            "Cmd {} / Node {}: subscribing, expecting {} events",
                            cmd_id,
                            node,
                            expected_cnt
                        );

                        let id = machines[node % n_nodes];
                        let client = ApiClient::from_machine(sim.machine(id), app_manifest(), None).unwrap();
                        let query = to_query(tags).to_string();
                        let request = SubscribeRequest {
                            lower_bound: None,
                            query,
                        };
                        async move {
                            let mut req = client.subscribe(request).await?;
                            let mut actual = 0;
                            if expected_cnt > 0 {
                                while tokio::time::timeout(Duration::from_secs(10), req.next())
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
                    }
                    .boxed(),
                })
                .collect::<FuturesUnordered<_>>();

            while let Some(res) = futs.next().await {
                res?;
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
        .tests(2)
        .gen(Gen::new(200))
        .quickcheck(interleaved as fn(TestInput) -> TestResult)
}

#[cfg(not(target_os = "linux"))]
fn main() {}
