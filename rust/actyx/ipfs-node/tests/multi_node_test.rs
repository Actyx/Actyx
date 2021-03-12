use anyhow::Result;
use futures::prelude::*;
use ipfs_node::{BlockAdapter, IpfsNode, NodeConfig, NodeIdentity};
use libp2p::multiaddr::{Multiaddr, Protocol};
use quickcheck::{QuickCheck, TestResult};
use std::{
    collections::BTreeMap,
    iter,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use tracing::*;
mod test_util;
use test_util::*;

/// Creates a graph of n independent ipfs nodes that can be addressed using memory transports
struct Graph {
    pub nodes: Vec<IpfsNode>,
}

static GLOBAL_MEM_TRANSPORT: AtomicU64 = AtomicU64::new(1);

impl Graph {
    /// Create a new graph.
    ///
    /// - `num_nodes` is the number of bootstrap nodes
    /// - `num_bootstrap` is the number of bootstrap nodes. Must be <= `num_nodes`. Set this to 0 to start the nodes disconnected.
    async fn new(num_nodes: usize, num_bootstrap: usize, use_mdns: bool) -> Self {
        assert!(num_bootstrap <= num_nodes);
        if num_nodes == 0 {
            panic!("expecting at least one node");
        }

        // let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        // make globally unique memory ports for the memory transports, and generate peer ids for them
        let peers = (0..num_nodes)
            .map(|_| {
                (
                    GLOBAL_MEM_TRANSPORT.fetch_add(1, Ordering::SeqCst),
                    NodeIdentity::generate(),
                )
            })
            .collect::<BTreeMap<u64, NodeIdentity>>();

        // ids and keypairs of the bootstrap nodes
        let bootstrap = peers.iter().take(num_bootstrap).collect::<BTreeMap<_, _>>();

        // make the nodes
        let mut nodes = Vec::with_capacity(num_nodes);
        for (port, keypair) in &peers {
            let node = build_node(*port, keypair.clone(), bootstrap.clone(), use_mdns)
                .await
                .unwrap();
            nodes.push(node);
        }
        Self { nodes }
    }

    fn peers(&self) -> Vec<Vec<String>> {
        self.nodes.iter().map(|node| node.peers()).collect()
    }

    async fn ensure_min_connected(&self, min_connected: usize) -> Result<()> {
        // wait for all nodes to be connected
        loop {
            let peers = self.peers().into_iter().map(|x| x.len()).collect::<Vec<_>>();
            let c = *peers.iter().min().unwrap();
            if c >= min_connected {
                return Ok(());
            } else {
                info!(
                    "Nodes not yet fully connected. Min peer count {} of {}",
                    c, min_connected
                );
                info!("Connectivity {:?}", peers);
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

async fn build_node(
    port: u64,
    key: NodeIdentity,
    bootstrap: BTreeMap<&u64, &NodeIdentity>,
    use_mdns: bool,
) -> Result<IpfsNode> {
    info!(
        "building node {:?} for port {} with {} bootstrap nodes and mdns {}",
        key.to_keypair().public(),
        port,
        bootstrap.len(),
        use_mdns
    );
    let mut config = NodeConfig::new(Default::default())?;
    config.enable_dev_transport = true;
    config.listen = vec![Protocol::Memory(port).into()];
    config.local_key = key;
    config.use_mdns = use_mdns;
    config.bootstrap = bootstrap
        .into_iter()
        .filter(|(p, _)| **p != port)
        .map(|(p, k)| {
            let mut addr: Multiaddr = Protocol::Memory(*p).into();
            addr.push(Protocol::P2p(k.to_keypair().public().into_peer_id().into()));
            addr
        })
        .collect::<Vec<_>>();
    IpfsNode::new(config).await
}

async fn test_discovery(num_nodes: usize, _seed: u64) -> Result<()> {
    let graph = Graph::new(num_nodes, 1, false).await;
    // wait for all nodes to be connected
    graph.ensure_min_connected(num_nodes - 1).await?;

    // done
    Ok(())
}

async fn test_bitswap(num_nodes: usize, _seed: u64) -> Result<()> {
    let graph = Graph::new(num_nodes, 1, true).await;
    // wait for all nodes to be connected
    graph.ensure_min_connected(num_nodes - 1).await?;

    let sender = num_nodes - 2;
    let receiver = num_nodes - 1;
    let block = random_block();
    info!("storing data on node {}", sender);
    graph.nodes[sender].insert(block.clone()).await?;
    info!("getting data from node {}", receiver);
    let data2 = graph.nodes[receiver].fetch(block.cid()).await?;

    assert_eq!(block.data(), data2.data());

    // done
    Ok(())
}

async fn test_bitswap_sync_chain(num_nodes: usize, _seed: u64) -> Result<()> {
    let graph = Graph::new(num_nodes, 1, true).await;
    // wait for all nodes to be connected
    graph.ensure_min_connected(num_nodes - 1).await?;

    let sender = &graph.nodes[num_nodes - 2];
    let receiver = &graph.nodes[num_nodes - 1];
    // sync a chain
    let (cid, blocks) = build_chain("chain", 1000)?;
    let size: usize = blocks.iter().map(|block| block.data().len()).sum();
    info!("chain built {} blocks, {} bytes", blocks.len(), size);
    info!("storing data on node {}", num_nodes - 2);
    for block in blocks.iter() {
        let block = libp2p_ax_bitswap::Block::new(block.data().to_vec(), *block.cid());
        sender.lock_store().put_block(&BlockAdapter(block), None)?;
    }
    info!("sync data from node {}", num_nodes - 1);
    let t0 = Instant::now();
    let _ = receiver
        .sync(&cid)
        .for_each(|x| async move { debug!("sync progress {:?}", x) })
        .await;
    info!(
        "chain sync complete {} ms {} blocks {} bytes!",
        t0.elapsed().as_millis(),
        blocks.len(),
        size
    );
    for block in blocks {
        let data = receiver.lock_store().get_block(block.cid())?;
        assert_eq!(data, Some(block.data().to_vec()));
    }

    // done
    Ok(())
}

async fn test_bitswap_sync_tree(num_nodes: usize, _seed: u64) -> Result<()> {
    let graph = Graph::new(num_nodes, 1, true).await;
    // wait for all nodes to be connected
    graph.ensure_min_connected(num_nodes - 1).await?;

    let sender = &graph.nodes[num_nodes - 2];
    let receiver = &graph.nodes[num_nodes - 1];
    // sync a tree
    info!("building a tree");
    let (cid, blocks) = build_tree("tree", 10, 4)?;
    let size: usize = blocks.iter().map(|block| block.data().len()).sum();
    info!("tree built {} blocks, {} bytes", blocks.len(), size);
    info!("storing data on node {}", num_nodes - 2);
    for block in blocks.iter() {
        let block = libp2p_ax_bitswap::Block::new(block.data().to_vec(), *block.cid());
        sender.lock_store().put_block(&BlockAdapter(block), None)?;
    }
    info!("sync data from node {}", num_nodes - 1);
    let t0 = Instant::now();
    let _ = receiver
        .sync(&cid)
        .for_each(|x| async move { debug!("sync progress {:?}", x) })
        .await;
    info!(
        "tree sync complete {} ms {} blocks {} bytes!",
        t0.elapsed().as_millis(),
        blocks.len(),
        size
    );
    for block in blocks {
        let data = receiver.lock_store().get_block(block.cid())?;
        assert_eq!(data, Some(block.data().to_vec()));
    }

    // done
    Ok(())
}

/// run a prop asynchronously on a fresh tokio runtime
async fn run_async_prop<F, R>(text: &'static str, num_nodes: usize, seed: u64, f: F) -> TestResult
where
    F: Fn(usize, u64) -> R + 'static,
    R: Future<Output = anyhow::Result<()>> + 'static,
{
    if !(2..=10).contains(&num_nodes) {
        return TestResult::discard();
    }
    let separator = iter::repeat('#').take(80).collect::<String>();

    // mark start of test
    info!("\n\n\n{}\nstart {} nodes={} seed={}", separator, text, num_nodes, seed);

    let res = f(num_nodes, seed).await;

    // mark actual end including shutdown
    info!("\n{}\nend {} nodes={} seed={}\n\n", separator, text, num_nodes, seed);

    // convert result to TestResult
    match res {
        Ok(_) => TestResult::passed(),
        Err(_) => TestResult::failed(),
    }
}

fn setup_logging() {
    tracing_subscriber::fmt().with_max_level(Level::ERROR).try_init().ok();
}

#[tokio::test(flavor = "multi_thread")]
async fn discovery_small() -> Result<()> {
    setup_logging();
    test_discovery(8, 0).await
}

#[tokio::test(flavor = "multi_thread")]
async fn bitswap_small() -> Result<()> {
    setup_logging();
    test_bitswap(3, 0).await
}

#[tokio::test(flavor = "multi_thread")]
async fn bitswap_sync_chain_small() -> Result<()> {
    setup_logging();
    test_bitswap_sync_chain(3, 0).await
}

#[tokio::test(flavor = "multi_thread")]
async fn bitswap_sync_tree_small() -> Result<()> {
    setup_logging();
    test_bitswap_sync_tree(3, 0).await
}

#[test]
fn bitswap_check() {
    setup_logging();
    fn prop(num_nodes: usize, seed: u64) -> TestResult {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(run_async_prop("test_bitswap", num_nodes, seed, test_bitswap))
    }

    QuickCheck::new()
        .max_tests(100)
        .quickcheck(prop as fn(usize, u64) -> TestResult)
}

#[test]
fn discovery_check() {
    setup_logging();
    fn prop(num_nodes: usize, seed: u64) -> TestResult {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(run_async_prop("test_discovery", num_nodes, seed, test_discovery))
    }

    QuickCheck::new()
        .max_tests(100)
        .quickcheck(prop as fn(usize, u64) -> TestResult)
}
