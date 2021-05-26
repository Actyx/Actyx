#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use netsim_embed::{Ipv4Range, Netsim};
    use std::net::Ipv4Addr;
    use swarm_cli::{Command, Config, Event};
    use swarm_harness::{MachineExt, MultiaddrExt};

    util::setup_logger();
    netsim_embed::unshare_user()?;
    async_global_executor::block_on(async move {
        let mut sim = Netsim::new();
        let net_a = sim.spawn_network(Ipv4Range::new(Ipv4Addr::new(192, 168, 0, 0), 24));
        let net_b = sim.spawn_network(Ipv4Range::new(Ipv4Addr::new(192, 168, 1, 0), 24));
        let net_c = sim.spawn_network(Ipv4Range::new(Ipv4Addr::new(192, 168, 2, 0), 24));
        sim.add_route(net_a, net_b);
        sim.add_route(net_a, net_c);
        let mut cfg = Config {
            path: None,
            node_name: None,
            keypair: 0,
            listen_on: vec!["/ip4/0.0.0.0/tcp/30000".parse().unwrap()],
            bootstrap: vec![],
            external: vec![],
            enable_mdns: false,
            enable_fast_path: false,
            enable_slow_path: false,
            enable_root_map: true,
            enable_discovery: true,
            enable_metrics: false,
        };
        let bootstrap = sim.spawn_machine(cfg.clone().into(), None).await;
        sim.plug(bootstrap, net_a, None).await;
        cfg.keypair = 1;
        let client = sim.spawn_machine(cfg.into(), None).await;
        sim.plug(client, net_b, None).await;

        for machine in sim.machines_mut() {
            loop {
                if let Some(Event::NewListenAddr(addr)) = machine.recv().await {
                    if !addr.is_loopback() {
                        break;
                    }
                }
            }
        }

        let bootstrap_id = sim.machine(bootstrap).peer_id();
        let bootstrap_addr = sim.machine(bootstrap).multiaddr();
        let client_id = sim.machine(client).peer_id();
        let client_addr = sim.machine(client).multiaddr();

        sim.machine(client)
            .send(Command::AddAddress(bootstrap_id, bootstrap_addr));

        loop {
            if let Some(Event::Connected(peer)) = sim.machine(bootstrap).recv().await {
                if peer == client_id {
                    break;
                }
            }
        }

        loop {
            if let Some(Event::NewExternalAddr(addr)) = sim.machine(client).recv().await {
                assert_eq!(addr, client_addr);
                break;
            }
        }

        sim.plug(client, net_c, None).await;
        let client_addr_new = sim.machine(client).multiaddr();

        loop {
            if let Some(Event::Disconnected(peer)) = sim.machine(bootstrap).recv().await {
                if peer == client_id {
                    break;
                }
            }
        }

        loop {
            if let Some(Event::Connected(peer)) = sim.machine(bootstrap).recv().await {
                if peer == client_id {
                    break;
                }
            }
        }

        let mut i = 0;
        while i < 3 {
            match sim.machine(client).recv().await {
                Some(Event::NewListenAddr(addr)) => {
                    assert_eq!(addr, client_addr_new);
                    i += 1;
                }
                Some(Event::ExpiredListenAddr(addr)) => {
                    assert_eq!(addr, client_addr);
                    i += 1;
                }
                Some(Event::NewExternalAddr(addr)) => {
                    assert_eq!(addr, client_addr_new);
                    i += 1;
                }
                _ => {}
            }
        }
    });
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {}
