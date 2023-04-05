#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use actyx_sdk::language::Query;
    use actyx_sdk::{tags, Payload};
    use async_std::future::timeout;
    use netsim_embed::{Ipv4Range, MachineId, Netsim, NetworkId};
    use std::path::Path;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    use std::{collections::BTreeMap, net::Ipv4Addr};
    use swarm_cli::{Command, Config, Event};
    use swarm_harness::MachineExt;
    use tempdir::TempDir;

    async fn spawn_machine(
        sim: &mut Netsim<Command, Event>,
        net: NetworkId,
        path: &Path,
        i: u64,
        name: &str,
        ro: bool,
    ) -> MachineId {
        let config = Config {
            path: Some(path.join(name)),
            node_name: Some(name.to_string()),
            keypair: i,
            listen_on: vec!["/ip4/0.0.0.0/tcp/3000".parse().unwrap()],
            bootstrap: vec![],
            external: vec![],
            enable_mdns: false,
            enable_fast_path: !ro,
            enable_slow_path: !ro,
            enable_root_map: !ro,
            enable_discovery: false,
            enable_metrics: false,
            enable_api: None,
            ephemeral_events: None,
            max_leaf_count: None,
            event_routes: Default::default(),
        };
        let machine = sim.spawn_machine(config.into(), None).await;
        sim.plug(machine, net, None).await;
        machine
    }

    swarm_harness::setup_env()?;
    let temp_dir = TempDir::new("read_only")?;
    async_global_executor::block_on(async move {
        let mut sim = Netsim::new();
        let net = sim.spawn_network(Ipv4Range::new(Ipv4Addr::new(192, 168, 0, 0), 24));
        let rw1 = spawn_machine(&mut sim, net, temp_dir.path(), 0, "rw1", false).await;
        let rw2 = spawn_machine(&mut sim, net, temp_dir.path(), 1, "rw2", false).await;
        let ro = spawn_machine(&mut sim, net, temp_dir.path(), 2, "ro", true).await;

        swarm_harness::fully_mesh(&mut sim, Duration::from_secs(20)).await?;
        tracing::info!("nodes started");

        for machine in sim.machines_mut() {
            machine.send(Command::GossipSubscribe("swarm-cli".into()));
            machine.send(Command::SubscribeQuery(Query::parse("FROM 'a'").unwrap()));
        }

        for machine in sim.machines_mut() {
            machine.send(Command::Append(vec![(
                tags!("a"),
                Payload::from_json_str(&format!("\"{}\"", machine.peer_id())).unwrap(),
            )]));
        }

        let gossip_per_peer: Arc<Mutex<BTreeMap<_, usize>>> = Default::default();

        tracing::info!("waiting for events");

        // first assert that the ro node's events are not propagated, but the others' are
        let ro_peer_id = sim.machine(ro).peer_id();
        let payload = Payload::from_json_str(&format!("\"{}\"", ro_peer_id)).unwrap();
        let start = Instant::now();
        for machine in sim.machines_mut() {
            let read_only = machine.peer_id() == ro_peer_id;
            let events = if read_only { 3 } else { 2 };
            for _ in 0..events {
                loop {
                    match timeout(Duration::from_secs(20), machine.recv()).await?.unwrap() {
                        Event::Result((_, _, payload2)) => {
                            if !read_only {
                                assert_ne!(payload2, payload);
                            }
                            break;
                        }
                        Event::GossipEvent(_, sender, _) => {
                            *gossip_per_peer.lock().unwrap().entry(sender).or_default() += 1;
                        }
                        _ => {}
                    }
                    anyhow::ensure!(start.elapsed() < Duration::from_secs(60), "Overall deadline exceeded");
                }
            }
        }

        tracing::info!("Listening in on the gossip traffic to check whether ro node is really silent");
        let m = sim.machine(rw1);
        let x = gossip_per_peer.clone();
        let _ = timeout(Duration::from_secs(15), async move {
            let mut g = x.lock().unwrap();
            loop {
                if let Some(Event::GossipEvent(_, sender, _)) = m.recv().await {
                    *g.entry(sender).or_default() += 1;
                }
            }
        })
        .await;
        let g = gossip_per_peer.lock().unwrap();
        assert_eq!(g.len(), 2);
        assert!(*g.get(&sim.machine(rw1).peer_id()).unwrap() > 0);
        assert!(*g.get(&sim.machine(rw2).peer_id()).unwrap() > 0);

        Ok(())
    })
}

#[cfg(not(target_os = "linux"))]
fn main() {}
