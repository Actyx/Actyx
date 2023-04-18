#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use actyx_sdk::{language::TagExpr, tags, LamportTimestamp, Offset, Payload, StreamNr};
    use async_std::future::timeout;
    use crypto::peer_id_to_node_id;
    use std::{
        convert::identity,
        str::FromStr,
        time::{Duration, Instant},
    };
    use structopt::StructOpt;
    use swarm_cli::{Command, Event, EventRoute, GossipMessage, RootMap, RootUpdate};
    use swarm_harness::{fully_meshed, HarnessOpts, MachineExt};

    swarm_harness::setup_env()?;
    let mut opts = HarnessOpts::from_args();
    opts.enable_fast_path = true;
    opts.enable_slow_path = true;
    opts.enable_root_map = true;
    opts.enable_discovery = false;
    opts.enable_metrics = false;
    let n_nodes = opts.n_nodes.max(2);
    opts.n_bootstrap = n_nodes;
    opts.n_nodes = n_nodes;
    opts.event_routes = vec![EventRoute::new(
        TagExpr::from_str("'test'").unwrap(),
        "test_stream".to_string(),
    )];
    const EVENTS: u64 = 100;
    swarm_harness::run_netsim(opts, |mut sim| async move {
        fully_meshed::<Event>(&mut sim, Duration::from_secs(60)).await?;

        for machine in sim.machines_mut() {
            machine.send(Command::GossipSubscribe("swarm-cli".into()));
        }

        let events = (0..EVENTS)
            .map(|i| (tags!("test"), Payload::from_json_str(&format!("{}", i)).unwrap()))
            .collect();

        let (first, rest) = sim.machines_mut().split_first_mut().unwrap();
        first.send(Command::Append(events));

        let start = Instant::now();

        let expectations = [
            (StreamNr::from(0), LamportTimestamp::from(2), Offset::from(1)),
            (
                StreamNr::from(1),
                LamportTimestamp::from(EVENTS + 2),
                Offset::from((EVENTS - 1) as u32),
            ),
        ];

        for machine in rest.iter_mut() {
            let mut received_test_root_update = false;
            let mut received_root_map = false;

            // RootUpdates go out whenever streams are updates
            // RootMaps go out every 10 seconds
            while let Some(x) = timeout(Duration::from_secs(15), machine.recv()).await? {
                if let Event::GossipEvent(_, sender, message) = x {
                    match message {
                        GossipMessage::RootUpdate(RootUpdate { stream, .. })
                            if stream.stream_nr() == StreamNr::from(0) =>
                        {
                            // The respective appends happen before the full initialization of the the banyan store
                            // This makes the RootMap test a bit slower because the other nodes will
                            // only be able to know about the default stream from RootMap
                            panic!("Should not publish the root update for the stream mappings.")
                        }
                        GossipMessage::RootUpdate(RootUpdate {
                            lamport,
                            offset,
                            stream,
                            ..
                        }) if stream.stream_nr() == StreamNr::from(1) => {
                            // RootUpdates only from `first`!
                            assert_eq!(sender, first.peer_id());
                            // 2 implicit mappings for default & test streams
                            assert_eq!(lamport, (EVENTS + 2).into());
                            // the offset starts at 0
                            assert_eq!(offset.unwrap(), Offset::from(EVENTS as u32 - 1));
                            received_test_root_update = true;
                        }
                        GossipMessage::RootUpdate(RootUpdate { stream, .. }) => {
                            panic!("Stream {} RootUpdate not handled!", stream);
                        }
                        GossipMessage::RootMap(RootMap {
                            entries,
                            lamport,
                            offsets,
                            ..
                        }) => {
                            // Ignore if the sender is the first,
                            // since of course the sender knows itself...
                            // That wouldn't test anything
                            if sender == first.peer_id() {
                                continue;
                            }
                            assert!(lamport >= EVENTS.into());

                            let mut root_map = [false, false];
                            // Added just to ensure that someone doesn't forget to change both
                            assert_eq!(expectations.len(), root_map.len());

                            for (stream_nr, expected_lamport, expected_offset) in expectations.iter() {
                                let stream_id =
                                    peer_id_to_node_id(first.peer_id()).unwrap().stream((*stream_nr).into());

                                let idx = entries.keys().enumerate().find_map(|(idx, stream)| {
                                    if *stream == stream_id {
                                        Some(idx)
                                    } else {
                                        None
                                    }
                                });

                                // We need to tolerate the fact that the RootMap may not always contain the streams we're expecting
                                // but at some point it MUST have them, so we just expect to eventually have them
                                if let Some(idx) = idx {
                                    let (offset, lamport_for_root) = offsets[idx];
                                    assert_eq!(lamport_for_root, *expected_lamport);
                                    assert_eq!(offset, *expected_offset);
                                    // Dirty hack to get the stream_nr as index
                                    root_map[u64::from(*stream_nr) as usize] = true;
                                } else {
                                    tracing::error!(
                                        "Checking machine {:?}, received RootMap does not contain stream {:?}",
                                        machine.id(),
                                        stream_id
                                    );
                                }
                            }

                            received_root_map = root_map.into_iter().all(identity);
                        }
                    }
                    if received_root_map && received_test_root_update {
                        break;
                    }
                    // Poor man's deadline
                    // Using timeout_at is a bit harder because we're waiting for several events over N machines and not a single event
                    anyhow::ensure!(start.elapsed() < Duration::from_secs(60), "Assertions took too long");
                }
            }
        }
        Ok(())
    })
}

#[cfg(not(target_os = "linux"))]
fn main() {}
