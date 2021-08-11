//! Tests that the network resolves partitions by randomly applying a set of partitions and
//! checking that all nodes that can be connected end up connected.

#[cfg(not(target_os = "linux"))]
fn main() {}

#[cfg(target_os = "linux")]
fn main() {
    use anyhow::Result;
    use async_std::future::timeout;
    use libp2p::multiaddr::{Multiaddr, Protocol};
    use netsim_embed::{Ipv4Range, Machine, MachineId, NetworkId};
    use petgraph::graph::{NodeIndex, UnGraph};
    use petgraph::visit::EdgeRef;
    use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
    use std::net::Ipv4Addr;
    use std::time::Duration;
    use swarm_cli::{Command, Config, Event, PeerId};
    use swarm_harness::MultiaddrExt;

    type Netsim = netsim_embed::Netsim<Command, Event>;

    fn health_test(input: HealthTest) -> TestResult {
        let res = async_global_executor::block_on(health_test_inner(input));
        match res {
            Ok(()) => TestResult::passed(),
            Err(e) => {
                tracing::error!("Error from run: {:#?}", e);
                TestResult::error(format!("{:#?}", e))
            }
        }
    }

    const NETWORKS: &[usize] = &[0, 1];
    const NODES: &[usize] = &[0, 1, 2];
    const ROUTES: &[(usize, usize)] = &[(0, 1)];

    #[derive(Clone, Debug)]
    enum Action {
        Join { node: usize, network: usize },
        EnableRoute { route: (usize, usize) },
        DisableRoute { route: (usize, usize) },
    }

    impl Arbitrary for Action {
        fn arbitrary(gen: &mut Gen) -> Self {
            if bool::arbitrary(gen) {
                let node = *gen.choose(NODES).unwrap() + NETWORKS.len();
                let network = *gen.choose(NETWORKS).unwrap();
                Self::Join { node, network }
            } else {
                let route = *gen.choose(ROUTES).unwrap();
                if bool::arbitrary(gen) {
                    Self::EnableRoute { route }
                } else {
                    Self::DisableRoute { route }
                }
            }
        }
    }

    #[derive(Clone, Debug)]
    struct HealthTest {
        networks: usize,
        nodes: usize,
        steps: Vec<Action>,
    }

    impl Arbitrary for HealthTest {
        fn arbitrary(gen: &mut Gen) -> Self {
            Self {
                networks: NETWORKS.len(),
                nodes: NODES.len(),
                steps: Arbitrary::arbitrary(gen),
            }
        }
    }

    #[derive(Debug)]
    struct Node {
        id: MachineId,
        node: NodeIndex<u32>,
        peer_id: PeerId,
    }

    #[derive(Debug)]
    struct Net {
        id: NetworkId,
        node: NodeIndex<u32>,
    }

    async fn wait_for_listen_addr(machine: &mut Machine<Command, Event>) -> Multiaddr {
        let addr = machine
            .select(|ev| {
                if let Event::NewListenAddr(addr) = ev {
                    if !addr.is_loopback() {
                        return Some(addr.clone());
                    }
                }
                None
            })
            .await
            .unwrap();
        tracing::info!("{} has addr {}", machine.id(), addr);
        addr
    }

    async fn health_test_inner(input: HealthTest) -> Result<()> {
        let mut sim = Netsim::new();
        let mut nets = Vec::with_capacity(NETWORKS.len());
        let mut nodes = Vec::with_capacity(NODES.len());
        let mut bootstrap = Vec::with_capacity(NETWORKS.len());
        let mut top = UnGraph::new_undirected();
        for net_id in NETWORKS {
            let peer_id = PeerId::from(swarm_cli::keypair(*net_id as _));
            let addr = format!("/ip4/192.168.{}.2/tcp/3000/p2p/{}", net_id, peer_id)
                .parse()
                .unwrap();
            bootstrap.push(addr);
        }
        for net_id in NETWORKS {
            let range = Ipv4Range::new(Ipv4Addr::new(192, 168, *net_id as u8, 0), 24);
            let net = sim.spawn_network(range);
            let peer_id = PeerId::from(swarm_cli::keypair(*net_id as _));
            let cfg = Config {
                path: None,
                node_name: None,
                keypair: *net_id as u64,
                listen_on: vec!["/ip4/0.0.0.0/tcp/3000".parse().unwrap()],
                bootstrap: bootstrap.clone(),
                external: vec![],
                enable_mdns: false,
                enable_discovery: true,
                enable_fast_path: true,
                enable_slow_path: false,
                enable_root_map: true,
                enable_metrics: false,
                enable_api: None,
                ephemeral_events: None,
                max_leaf_count: None,
            };
            let machine = sim.spawn_machine(cfg.into(), None).await;
            tracing::info!("{} is {}", machine, peer_id);
            sim.plug(machine, net, None).await;
            tracing::info!("{} joining {:?}", machine, net);
            let idx = top.add_node(());
            nodes.push(Node {
                id: machine,
                node: idx,
                peer_id,
            });
            nets.push(Net { id: net, node: idx });
            let mut addr = wait_for_listen_addr(sim.machine(machine)).await;
            addr.push(Protocol::P2p(peer_id.into()));
            bootstrap.push(addr);
        }
        for node in NODES {
            let node = node + nets.len();
            let peer_id = swarm_cli::keypair(node as u64).into();
            let cfg = Config {
                path: None,
                node_name: None,
                keypair: node as u64,
                listen_on: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()],
                bootstrap: bootstrap.clone(),
                external: vec![],
                enable_mdns: false,
                enable_discovery: true,
                enable_fast_path: true,
                enable_slow_path: false,
                enable_root_map: true,
                enable_metrics: false,
                enable_api: None,
                ephemeral_events: None,
                max_leaf_count: None,
            };
            let machine = sim.spawn_machine(cfg.into(), None).await;
            tracing::info!("{} is {}", machine, peer_id);
            let idx = top.add_node(());
            nodes.push(Node {
                id: machine,
                node: idx,
                peer_id,
            });
            let network = node % nets.len();
            let net = nets[network].id;
            sim.plug(machine, net, None).await;
            tracing::info!("{} joining {:?}", machine, net);
            top.add_edge(nodes[node].node, nets[network].node, ());
            wait_for_listen_addr(sim.machine(machine)).await;
        }
        for (a, b) in ROUTES {
            sim.add_route(nets[*a].id, nets[*b].id);
            top.add_edge(nets[*a].node, nets[*b].node, ());
        }
        swarm_harness::fully_meshed(&mut sim, Duration::from_secs(20)).await?;
        tracing::info!("fully meshed");
        for step in input.steps {
            let mut next_top = top.clone();
            let node = match step {
                Action::Join { node, network } => {
                    if top
                        .edges_connecting(nodes[node].node, nets[network].node)
                        .next()
                        .is_some()
                    {
                        continue;
                    }
                    let node_id = nodes[node].id;
                    let net_id = nets[network].id;
                    tracing::info!("{} joining {:?}", node_id, net_id);
                    sim.plug(node_id, net_id, None).await;
                    for edge in top.edges(nodes[node].node) {
                        next_top.remove_edge(edge.id());
                    }
                    next_top.add_edge(nodes[node].node, nets[network].node, ());
                    wait_for_listen_addr(sim.machine(node_id)).await;
                    Some(nodes[node].node)
                }
                Action::EnableRoute { route: (a, b) } => {
                    if top.edges_connecting(nets[a].node, nets[b].node).next().is_some() {
                        continue;
                    }
                    tracing::info!("enabling route {:?} {:?}", nets[a].id, nets[b].id);
                    sim.enable_route(nets[a].id, nets[b].id);
                    next_top.add_edge(nets[a].node, nets[b].node, ());
                    None
                }
                Action::DisableRoute { route: (a, b) } => {
                    if top.edges_connecting(nets[a].node, nets[b].node).next().is_none() {
                        continue;
                    }
                    tracing::info!("disabling route {:?} {:?}", nets[a].id, nets[b].id);
                    sim.disable_route(nets[a].id, nets[b].id);
                    for edge in top.edges_connecting(nets[a].node, nets[b].node) {
                        next_top.remove_edge(edge.id());
                    }
                    None
                }
            };
            let mut events = vec![];
            for n1 in &nodes {
                for n2 in &nodes {
                    if n1.node == n2.node {
                        continue;
                    }
                    let before = petgraph::algo::has_path_connecting(&top, n1.node, n2.node, None);
                    let after = petgraph::algo::has_path_connecting(&next_top, n1.node, n2.node, None);
                    let changed = node.into_iter().any(|n| n == n1.node || n == n2.node);
                    match (before, after, changed) {
                        // In some cases the roaming peers will reconnect to a peer before the
                        // peer noticed it was disconnected.
                        /*(true, true, true) => {
                            events.push((n1, n2, false));
                            events.push((n1, n2, true));
                        }*/
                        (true, false, _) => {
                            events.push((n1, n2, false));
                        }
                        (false, true, _) => {
                            events.push((n1, n2, true));
                        }
                        _ => {}
                    }
                }
            }
            let mut timedout = false;
            for (a, b, is_connected) in events {
                if is_connected {
                    tracing::info!("waiting for {} to connect to {}", a.id, b.id);
                } else {
                    tracing::info!("waiting for {} to disconnect from {}", a.id, b.id);
                }
                let id = a.id;
                let id2 = b.id;
                let peer_id = b.peer_id;
                let fut = sim.machine(id).select(|ev| {
                    match ev {
                        Event::Connected(peer_id2) => {
                            if peer_id == *peer_id2 {
                                tracing::info!("{} connected to {}", id, id2);
                                return Some(is_connected);
                            }
                        }
                        Event::Disconnected(peer_id2) => {
                            if peer_id == *peer_id2 {
                                tracing::info!("{} disconnected from {}", id, id2);
                                return Some(!is_connected);
                            }
                        }
                        _ => {}
                    }
                    None
                });
                match timeout(Duration::from_secs(120), fut).await {
                    Ok(Some(true)) => {}
                    Ok(_) => panic!(),
                    Err(_) => {
                        timedout = true;
                    }
                }
            }
            for machine in sim.machines_mut() {
                for ev in machine.drain() {
                    match ev {
                        Event::Connected(peer_id) => {
                            let n = nodes.iter().find(|n| n.peer_id == peer_id).unwrap();
                            tracing::error!("{} connected to {}", machine.id(), n.id);
                        }
                        Event::Disconnected(peer_id) => {
                            let n = nodes.iter().find(|n| n.peer_id == peer_id).unwrap();
                            tracing::error!("{} disconnected from {}", machine.id(), n.id);
                        }
                        _ => {}
                    }
                }
            }
            if timedout {
                panic!("timeout");
            }
            top = next_top;
        }
        tracing::info!("test passed\n");
        Ok(())
    }

    swarm_harness::setup_env().unwrap();
    QuickCheck::new()
        .gen(Gen::new(10))
        .tests(10)
        .quickcheck(health_test as fn(HealthTest) -> TestResult)
}
