#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use actyx_sdk::{tags, Offset, Payload};
    use async_std::future::timeout;
    use crypto::peer_id_to_node_id;
    use std::time::{Duration, Instant};
    use structopt::StructOpt;
    use swarm_cli::{Command, Event, GossipMessage, RootMap, RootUpdate};
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
    const EVENTS: u64 = 100;
    swarm_harness::run_netsim(opts, |mut sim| async move {
        fully_meshed::<Event>(&mut sim, Duration::from_secs(60)).await?;

        for machine in sim.machines_mut() {
            machine.send(Command::GossipSubscribe("swarm-cli".into()));
        }

        let events = (0..EVENTS)
            .map(|i| (tags!("test"), Payload::from_json_str(&*format!("{}", i)).unwrap()))
            .collect();

        let (first, rest) = sim.machines_mut().split_first_mut().unwrap();
        first.send(Command::Append(0.into(), events));

        let start = Instant::now();
        for m in rest.iter_mut() {
            let mut received_root_update = false;
            let mut received_root_map = false;
            while let Some(x) = timeout(Duration::from_secs(15), m.recv()).await? {
                if let Event::GossipEvent(_, sender, message) = x {
                    match message {
                        GossipMessage::RootUpdate(RootUpdate {
                            lamport,
                            offset,
                            stream,
                            ..
                        }) if stream.stream_nr() == 0.into() => {
                            // RootUpdates only from `first`!
                            assert_eq!(sender, first.peer_id());
                            assert_eq!(lamport, EVENTS.into());
                            assert_eq!(offset.unwrap(), Offset::from(EVENTS as u32 - 1));
                            received_root_update = true;
                        }
                        GossipMessage::RootMap(RootMap {
                            entries,
                            lamport,
                            offsets,
                            ..
                        }) => {
                            assert!(lamport >= EVENTS.into());
                            let s = peer_id_to_node_id(first.peer_id()).unwrap().stream(0.into());
                            let idx = entries
                                .keys()
                                .enumerate()
                                .find_map(|(idx, stream)| if *stream == s { Some(idx) } else { None })
                                .unwrap();
                            let (offset, lamport_for_root) = offsets[idx];
                            assert_eq!(lamport_for_root, EVENTS.into());
                            assert_eq!(offset, Offset::from(EVENTS as u32 - 1));
                            received_root_map = true;
                        }
                        _ => {}
                    }
                    if received_root_map && received_root_update {
                        break;
                    }
                    anyhow::ensure!(start.elapsed() < Duration::from_secs(60), "Assertions took too long");
                }
            }
        }
        Ok(())
    })
}

#[cfg(not(target_os = "linux"))]
fn main() {}
