#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use netsim_embed::{Ipv4Range, Netsim};
    use std::net::Ipv4Addr;
    use swarm_cli::{Command, Config, Event};
    use swarm_harness::{MachineExt, MultiaddrExt};
    use tempdir::TempDir;

    swarm_harness::setup_env()?;
    let temp_dir = TempDir::new("swarm-harness")?;
    async_global_executor::block_on(async move {
        let mut sim = Netsim::new();
        let net_a = sim.spawn_network(Ipv4Range::new(Ipv4Addr::new(192, 168, 0, 0), 24));
        let net_b = sim.spawn_network(Ipv4Range::new(Ipv4Addr::new(192, 168, 1, 0), 24));
        let net_c = sim.spawn_network(Ipv4Range::new(Ipv4Addr::new(192, 168, 2, 0), 24));
        sim.add_route(net_a, net_b);
        sim.add_route(net_a, net_c);
        sim.add_route(net_b, net_c);
        for (i, net) in [net_a, net_b, net_c].iter().enumerate() {
            let cfg = Config {
                path: Some(temp_dir.path().join(i.to_string())),
                node_name: None,
                keypair: i as _,
                listen_on: vec!["/ip4/0.0.0.0/tcp/30000".parse().unwrap()],
                bootstrap: vec![],
                external: vec![],
                enable_mdns: false,
                enable_fast_path: false,
                enable_slow_path: false,
                enable_root_map: true,
                enable_discovery: true,
                enable_metrics: false,
                enable_api: None,
                ephemeral_events: None,
                max_leaf_count: None,
            };
            let machine = sim.spawn_machine(cfg.into(), None).await;
            sim.plug(machine, *net, None).await;
        }

        for machine in sim.machines_mut() {
            loop {
                if let Some(Event::NewListenAddr(addr)) = machine.recv().await {
                    if !addr.is_loopback() {
                        break;
                    }
                }
            }
        }

        tracing::info!("nodes started");

        let mut machines = sim.machines_mut().chunks_mut(1);
        let a = &mut machines.next().unwrap()[0];
        let b = &mut machines.next().unwrap()[0];
        let c = &mut machines.next().unwrap()[0];
        let a_id = a.peer_id();
        let b_id = b.peer_id();
        let c_id = c.peer_id();
        let a_addr = a.multiaddr();

        b.send(Command::AddAddress(a_id, a_addr.clone()));
        c.send(Command::AddAddress(a_id, a_addr));

        loop {
            let event = b.recv().await;
            tracing::info!("{:?}", event);
            if let Some(Event::Connected(peer)) = event {
                if peer == c_id {
                    break;
                }
            }
        }

        tracing::info!("nodes connected to `c`");

        loop {
            if let Some(Event::Connected(peer)) = c.recv().await {
                if peer == b_id {
                    break;
                }
            }
        }

        Ok(())
    })
}

#[cfg(not(target_os = "linux"))]
fn main() {}
