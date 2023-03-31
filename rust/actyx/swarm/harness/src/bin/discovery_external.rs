#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use libp2p::multiaddr::Protocol;
    use netsim_embed::{Ipv4Range, Netsim};
    use std::{net::Ipv4Addr, time::Duration};
    use swarm_cli::{Command, Config, Event};
    use swarm_harness::{m, select_multi, select_single, selector, MachineExt, MultiaddrExt};

    swarm_harness::setup_env()?;
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
            enable_api: None,
            ephemeral_events: None,
            max_leaf_count: None,
            event_routes: Default::default(),
        };
        let bootstrap = sim.spawn_machine(cfg.clone().into(), None).await;
        sim.plug(bootstrap, net_a, None).await;
        cfg.keypair = 1;
        let client = sim.spawn_machine(cfg.into(), None).await;
        sim.plug(client, net_b, None).await;

        for machine in sim.machines_mut() {
            select_single(
                machine,
                Duration::from_secs(1),
                |ev| m!(ev, Event::NewListenAddr(addr) if !addr.is_loopback() => ()),
            )
            .await;
        }

        let bootstrap_id = sim.machine(bootstrap).peer_id();
        let bootstrap_addr = sim.machine(bootstrap).multiaddr();
        let client_id = sim.machine(client).peer_id();
        let client_addr = sim.machine(client).multiaddr();
        let mut client_addr_p2p = client_addr.clone();
        client_addr_p2p.push(Protocol::P2p(client_id.into()));

        sim.machine(client)
            .send(Command::AddAddress(bootstrap_id, bootstrap_addr));

        select_single(
            sim.machine(bootstrap),
            Duration::from_secs(3),
            |ev| m!(ev, Event::Connected(peer) if *peer == client_id => ()),
        )
        .await;
        let addr = select_single(
            sim.machine(client),
            Duration::from_secs(3),
            |ev| m!(ev, Event::NewExternalAddr(addr) => addr.clone()),
        )
        .await;
        assert_eq!(addr, client_addr_p2p);

        sim.plug(client, net_c, None).await;
        let client_addr_new = sim.machine(client).multiaddr();
        let mut client_addr_new_p2p = client_addr_new.clone();
        client_addr_new_p2p.push(Protocol::P2p(client_id.into()));

        select_single(
            sim.machine(bootstrap),
            Duration::from_secs(90),
            |ev| m!(ev, Event::Disconnected(peer) if *peer == client_id => ()),
        )
        .await;

        select_single(
            sim.machine(bootstrap),
            Duration::from_secs(3),
            |ev| m!(ev, Event::Connected(peer) if *peer == client_id => ()),
        )
        .await;

        select_multi(
            sim.machine(client),
            Duration::from_secs(3),
            vec![
                selector(|ev| m!(ev, Event::NewListenAddr(addr) if !addr.is_loopback() => assert_eq!(addr, &client_addr_new))),
                selector(|ev| m!(ev, Event::ExpiredListenAddr(addr) => assert_eq!(addr, &client_addr))),
                selector(|ev| m!(ev, Event::NewExternalAddr(addr) => assert_eq!(addr, &client_addr_new_p2p))),
            ],
        )
        .await;
    });
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn main() {}
