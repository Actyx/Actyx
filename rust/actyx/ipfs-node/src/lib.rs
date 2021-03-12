//! # Ipfs Node
//!
//! The purpose of this crate is to implement a node that implements a number of protocols in a
//! way that is interoperable with go-ipfs. This crate also provides an implementation of the ipfs
//! traits.
//!
//! ## Overall architecture of a rust-libp2p based node
//!
//! Despite exposing an interface based on new rust futures, rust-libp2p internally does not rely
//! on futures or streams. The central entity managing the behaviour of a node is a swarm, which is
//! constructed by combining several behaviours. Every behaviour contains the (mutable) state for an
//! aspect of the overall node behaviour. The behaviours could theoretically be combined manually.
//! Just to avoid boilerplate, they are combined using the `#[derive(NetworkBehaviour)]` macro.
//!
//! A behaviour can emit events that influence the overall swarm (e.g. dial a node or report
//! addresses), and can emit generic events that are interpreted at the level of the combined
//! behaviour. See [NetworkBehaviourAction].
//! But there is no generic mechanism for a behaviour to collaborate with another
//! behaviour. Essentially a behaviour is a bit of functionality with wires hanging out. All the
//! plumbing between behaviours needs to be done manually at the top level behaviour. This is why
//! even the tiny [examples] in the libp2p project are sometimes rather complex.
//!
//! Integration of the sub-behaviours with the swarm is done by implementing [NetworkBehaviourEventProcess](https://docs.rs/libp2p/*/libp2p/swarm/trait.NetworkBehaviourEventProcess.html)
//! for each event type of the sub-behaviours for the main behaviour. Again, since the inject_event method
//! gets a mutable reference to the main behaviour and therefore the entire user visible state of the swarm,
//! it is perfectly OK to directly call methods on other behaviours or modify the state.
//!
//! Callbacks into a behaviour are synchronous, often with a void return type. Since you get a
//! mutable reference to the behaviour state, it is perfectly fine to just update your state
//! immediately.
//!
//! The libp2p swarm needs to be polled with a [poll function],
//! similar to a futures stream to make progress.
//! Only this will update things like connectivity states and in turn call poll on the
//! main behaviour, which then will call poll on all its sub-behaviours.
//!
//! Note: The idiomatic way a rust-libp2p app wants to be written might seem very low level compared
//! to an approach that relies on stream combinators, but there is no way to avoid this style if you
//! want to work with the library.
//!
//! ## Implemented behaviours
//!
//! The node implements the identify, mdns and ping protocols to be visible from and to discover
//! other ipfs nodes.
//!
//! In addition, it implements the standard bitswap protocol for bulk data exchange and the
//! standard gossipsub protocol for low latency data interchange with other nodes.
//!
//! The discovery protocol is a custom ax specific protocol for peer discovery that does not rely
//! on a DHT.
//!
//! Compared to a typical ipfs node, we do not implement node or content discovery via the kademlia
//! DHT. Instead, we rely on a custom gossipsub based protocol for peer discovery, and full
//! connectivity between nodes whenever possible for content discovery. This is viable for a few
//! 100 nodes for TCP based connections and for a few 1000 nodes once we use QUIC for connections.
//! This should be sufficient for the medium term.
//!
//! At this time, our behaviour hierarchy is flat. If we need more behaviours we might want to switch
//! to a more complex tree like behaviour hierarchy, e.g. by grouping related behaviours.
//!
//! ### Cookbook
//!
//! An example how to interact with the overall swarm from a behaviour can be found in the [discovery behaviour].
//! The discovery behaviour will occasionally instruct the swarm to dial new nodes.
//!
//! An example how to use rust futures from inside a behaviour can be found inside bitswap, where
//! the AskMorePeers event triggers a callback. Another way to integrate streams and behaviours would
//! be to have a stream as a member of a behaviour, and then call poll for the stream from the poll
//! method of the behaviour.
//!
//! ## The ipfs node
//! The ipfs node implementation is a complex object that is polled from tokio. As a user of one of the ipfs traits,
//! you get an ipfs node *handle*, which is basically just an Arc. It is basically free to clone and allows you to
//! access the actual ipfs node.
//!
//! Any code that has been written to use the ipfs traits should in theory work unchanged with an ipfs
//! node handle, except that some traits have only stub implementations.
//!
//! [discovery behaviour]: ../src/ipfs_node/discovery/mod.rs.html#143
//! [poll function]: https://docs.rs/libp2p/*/libp2p/swarm/trait.NetworkBehaviour.html#tymethod.poll
//! [examples]: https://github.com/libp2p/rust-libp2p/tree/master/examples
//! [NetworkBehaviourAction]: https://docs.rs/libp2p/*/libp2p/swarm/enum.NetworkBehaviourAction.html
//!
//! # Diagnostics
//!
//! A rust-ipfs node provides an unstable method to get some diagnostics info. This data
//! can be accessed via `curl localhost:4457/_internal/swarm/state` for an actyxos node running
//! on localhost.
//!
//! The info returned from that endpoint is just json and is subject to change at any time.
//!
//! ## Example
//!
//!```json
//!  {
//!    "store": {
//!      "block_count": 123,
//!      "block_size": 456123
//!    },
//!    "swarm": {
//!      "listen_addrs": [
//!        "/ip4/127.0.0.1/tcp/4001",
//!        "/ip4/172.17.0.2/tcp/4001",
//!        "/ip4/172.26.0.1/tcp/4001"
//!      ],
//!      "peer_id": "12D3KooWHhbGYPu4kXfp3iNJq54RNFHA8SZk29vgNQwQ5zYJL5x1",
//!      "peers": {
//!        "QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH": {
//!          "addresses": {
//!            "/dns4/demo-bootstrap.actyx.net/tcp/4001": {
//!              "provenance": "Bootstrap",
//!              "state": {
//!                "Connected": {
//!                  "since": 33423
//!                }
//!              }
//!            },
//!            "/ip4/127.0.0.1/tcp/4001": {
//!              "provenance": "Swarm",
//!              "state": "Initial"
//!            },
//!            "/ip4/172.18.0.3/tcp/4001": {
//!              "provenance": "Swarm",
//!              "state": {
//!                "Disconnected": {
//!                  "since": 33423
//!                }
//!              }
//!            }
//!          },
//!          "connection_state": "Connected"
//!        }
//!      }
//!    }
//!  }
//!```
//!
//! The store section gives some very high level overview over the store. The number
//! of blocks in the store, as well as the total number of bytes.
//!
//! The swarm section gives info about the node itself (peer id and listen addresses),
//! as well as the nodes the node currently knows and its connection state.
//!
//! The `since` field gives the time in seconds since the last state change. The
//! `provenance` field gives information about where the address knowledge comes from.
#![deny(clippy::future_not_send)]

mod behaviour;
pub mod block_store;
mod discovery;
mod node_config;
mod sync;
mod transport;
mod unixfsv1;

pub use crate::behaviour::StoreResponse;
pub use crate::block_store::{BlockAdapter, BlockStore};
pub use crate::discovery::SwarmState;
pub use crate::node_config::{NodeConfig, NodeIdentity};
pub use crate::sync::SyncProgress;
pub use crate::transport::{build_dev_transport, build_transport};
pub use libipld::cid::Cid;
pub use libp2p_ax_bitswap as bitswap;
pub use libp2p_broadcast::Topic;

use crate::behaviour::Behaviour;
use crate::sync::{SyncId, SyncStream, Syncer};
use anyhow::Result;
use futures::{channel::mpsc, channel::oneshot, future, pin_mut, prelude::*, FutureExt};
use libp2p::{gossipsub, swarm::SwarmBuilder, Multiaddr, PeerId, Swarm};
use maplit::btreeset;
use parking_lot::Mutex;
use std::{
    collections::{BTreeSet, VecDeque},
    fmt,
    sync::Arc,
    task::Poll,
};
use tracing::*;

pub type Block = libipld::block::Block<libipld::store::DefaultParams>;

#[derive(Clone)]
pub struct IpfsNode {
    swarm: Arc<Mutex<Swarm<Behaviour>>>,
    block_store: Arc<Mutex<ipfs_sqlite_block_store::BlockStore>>,
    handles: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
}

impl fmt::Debug for IpfsNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IpfsNode").finish()
    }
}

impl IpfsNode {
    pub async fn test() -> Result<Self> {
        let mut config = NodeConfig::new(Default::default())?;
        config.block_store_size = 0;
        config.enable_dev_transport = true;
        config.listen = vec!["/memory/0".parse().unwrap()];
        Self::new(config).await
    }

    pub async fn new(config: NodeConfig) -> Result<Self> {
        let kp = config.local_key.to_keypair();
        let transport = if config.enable_dev_transport {
            build_dev_transport(kp.clone(), config.upgrade_timeout).await?
        } else {
            build_transport(kp.clone(), config.pre_shared_key, config.upgrade_timeout).await?
        };
        let public_key = kp.public();
        let bs = BlockStore::new(config.block_store_path, config.block_store_size)?;
        let block_store = bs.inner().clone();
        let mut blocks_added_stream = bs.blocks_added_stream();
        let behaviour = behaviour::Behaviour::new(
            kp,
            config.gossipsub_config,
            config.ping_config,
            bs,
            config.use_mdns,
            config.allow_publish,
        )
        .await?;
        let mut swarm = SwarmBuilder::new(transport, behaviour, public_key.into())
            .executor(Box::new(|fut| {
                tokio::task::spawn(fut);
            }))
            .build();
        for addr in config.listen {
            debug!("Swarm services trying to bind to {}", addr);
            Swarm::listen_on(&mut swarm, addr)?;
        }
        // add bootstrap nodes and immediately dial them
        for addr in config.bootstrap {
            swarm.discovery.state.add_bootstrap(addr);
        }
        // add announce addresses
        for addr in config.announce {
            swarm.discovery.state.add_announce(addr);
        }
        swarm.discovery.prune_and_dial_disconnected_peers();
        let node = Self {
            swarm: Arc::new(Mutex::new(swarm)),
            block_store,
            handles: Default::default(),
        };
        let mut handles = vec![];

        let node1 = node.clone();
        handles.push(tokio::task::spawn(async move {
            while let Some(added) = blocks_added_stream.next().await {
                node1.blocks_added(added);
            }
        }));
        handles.push(tokio::task::spawn(
            node.poll_repeated().inspect(|_| error!("swarm poll terminated")),
        ));
        *node.handles.lock() = handles;
        Ok(node)
    }

    fn poll(&self) -> impl Future<Output = ()> + 'static {
        let this = self.clone();
        future::poll_fn(move |context| {
            let mut state = this.lock();
            while let Poll::Ready(ev) = {
                let fut = state.next_event();
                pin_mut!(fut);
                fut.poll(context)
            } {
                state.process_swarm_event(ev);
            }
            Poll::Pending
        })
    }

    fn poll_repeated(&self) -> impl Future<Output = ()> + 'static {
        let this = self.clone();
        async move {
            loop {
                this.poll().await;
            }
        }
    }

    fn lock(&self) -> impl std::ops::DerefMut<Target = Swarm<Behaviour>> + '_ {
        self.swarm.lock()
    }

    pub fn lock_store(&self) -> impl std::ops::DerefMut<Target = ipfs_sqlite_block_store::BlockStore> + '_ {
        self.block_store.lock()
    }

    pub fn store(&self) -> Arc<Mutex<ipfs_sqlite_block_store::BlockStore>> {
        self.block_store.clone()
    }

    pub fn local_peer_id(&self) -> PeerId {
        *self.lock().discovery.state.peer_id()
    }

    pub fn listeners(&self) -> Vec<Multiaddr> {
        let swarm = self.lock();
        Swarm::listeners(&swarm).cloned().collect()
    }

    pub fn peers(&self) -> Vec<String> {
        self.lock()
            .discovery
            .state
            .connected_peers()
            .into_iter()
            .map(|x| x.to_string())
            .collect()
    }

    pub fn connect(&self, mut addr: Multiaddr) {
        crate::discovery::strip_peer_id(&mut addr);
        self.connect_many(vec![addr])
    }

    pub fn connect_many(&self, addresses: Vec<Multiaddr>) {
        let mut state = self.lock();
        for addr in addresses {
            if Swarm::dial_addr(&mut state, addr).is_err() {
                error!("tried to dial invalid address");
            }
        }
    }

    pub async fn fetch(&self, cid: &Cid) -> Result<Block> {
        let cid = Cid::new_v1(cid.codec(), *cid.hash());
        debug!("get {:x} {}", cid.codec(), cid);
        let block_store = { self.lock().block_store.clone() };
        let local_block = block_store.get_block(cid).await?;
        let block = if let Some(block) = local_block {
            block
        } else {
            let (sender, receiver) = oneshot::channel::<libp2p_ax_bitswap::Block>();
            {
                let mut state = self.lock();
                state.bitswap.want_blocks(btreeset! { cid });
                state.block_listeners.register(cid, sender);
            };
            receiver.await?
        };
        Ok(Block::new_unchecked(*block.cid(), block.data().to_vec()))
    }

    pub async fn insert(&self, block: Block) -> Result<()> {
        let (cid, data) = block.into_inner();
        let bs = { self.lock().block_store.clone() };
        bs.put_block(libp2p_ax_bitswap::Block::new(data, cid)).await?;
        Ok(())
    }

    pub fn sync(&self, cid: &Cid) -> SyncStream {
        let mut state = self.lock();
        let mut sync = &mut state.sync_states;
        let id = SyncId(sync.next_id);
        sync.next_id += 1;
        let (tx, rx) = mpsc::unbounded();
        sync.current.insert(id, Syncer::new(*cid, tx));
        self.get_missing_blocks(id, cid, state.block_store.clone());
        SyncStream::new(self.swarm.clone(), id, rx)
    }

    fn blocks_added(&self, cids: Vec<Cid>) {
        debug!(
            "blocks added into store {:?}",
            cids.iter().map(|cid| cid.to_string()).collect::<Vec<_>>().join(",")
        );
        let mut state = self.lock();
        let bs = state.block_store.clone();
        for (id, syncer) in state.sync_states.current.iter_mut() {
            let missing = &mut syncer.missing;
            let mut removed = 0usize;
            for cid in &cids {
                if missing.remove(cid) {
                    removed += 1;
                }
            }
            // this is breadth first traversal. We only go deep once we have exhausted going wide.
            // if we would do this after the first update, we would get depth first traversal.
            if missing.is_empty() {
                self.get_missing_blocks(*id, &syncer.root, bs.clone());
            }
            if removed > 0 {
                syncer.send_progress(SyncProgress::BlocksReceived(removed));
            }
        }
    }

    /// call get_missing_blocks on the block store, and call set_missing_blocks with the result
    fn get_missing_blocks(&self, id: SyncId, cid: &Cid, bs: BlockStore) {
        let node = self.clone();
        tokio::task::spawn(
            bs.get_missing_blocks(*cid)
                .map(move |result| node.set_missing_blocks(id, result)),
        );
    }

    // called by get_missing_blocks when we got info about missing blocks
    fn set_missing_blocks(&self, id: SyncId, result: Result<BTreeSet<libipld::Cid>>) {
        let mut state = self.lock();
        match result {
            Ok(missing) => {
                if missing.is_empty() {
                    if let Some(mut syncer) = state.sync_states.current.remove(&id) {
                        syncer.send_progress(SyncProgress::Done);
                    }
                } else if let Some(syncer) = state.sync_states.current.get_mut(&id) {
                    syncer.send_progress(SyncProgress::MissingBlocksFound(missing.len()));
                    syncer.missing = missing.clone();
                    // these blocks were missing just a short time ago, so we just ask
                    // for them all again via bitswap. There is a tiny chance that a block
                    // has just been added, but there is not much we can do about that.
                    // we will get an answer even for blocks that are re-added, so as long
                    // as we get it it's all good.
                    debug!(
                        "bitswap.want_blocks {}",
                        missing.iter().map(|cid| cid.to_string()).collect::<Vec<_>>().join(",")
                    );
                    state.bitswap.want_blocks(missing);
                }
            }
            Err(cause) => {
                if let Some(syncer) = state.sync_states.current.remove(&id) {
                    syncer.send_abort(cause);
                }
            }
        }
    }

    pub async fn gc(&self) -> Result<()> {
        // this just gets hold of the block store inside the lock.
        let bs = { self.lock().block_store.clone() };
        bs.gc().await
    }

    pub async fn alias_many(&self, aliases: Vec<(Vec<u8>, Option<Cid>)>) -> Result<()> {
        let bs = { self.lock().block_store.clone() };
        bs.alias_many(aliases).await
    }

    pub fn cat(&self, cid: Cid, path: VecDeque<String>) -> impl Stream<Item = Result<Vec<u8>>> + Send {
        unixfsv1::UnixfsStream::new(unixfsv1::UnixfsDecoder::new(self.clone(), cid, path))
    }

    pub fn publish(&self, topic: &str, data: Vec<u8>) -> Result<()> {
        if !self.lock().allow_publish {
            return Ok(());
        }
        let topic = gossipsub::IdentTopic::new(topic);
        debug_assert!({
            Topic::new(topic.hash().as_str().as_ref());
            true
        });
        self.lock()
            .gossipsub
            .publish(topic, data)
            .map_err(|err| anyhow::anyhow!("{:?}", err))?;
        Ok(())
    }

    pub fn broadcast(&self, topic: &str, data: Vec<u8>) -> Result<()> {
        let topic = gossipsub::IdentTopic::new(topic);
        let topic = Topic::new(topic.hash().as_str().as_ref());
        self.lock().broadcast.broadcast(&topic, data.into());
        Ok(())
    }

    pub fn subscribe(&self, topic: &str) -> Result<impl Stream<Item = Vec<u8>>> {
        let ident_topic = gossipsub::IdentTopic::new(topic);
        let topic_name = topic.to_string();
        let topic = Topic::new(ident_topic.hash().as_str().as_ref());
        let (sender, receiver) = mpsc::unbounded();
        {
            let mut state = self.lock();
            state.topic_listeners.register(topic, sender);
            state
                .gossipsub
                .subscribe(&ident_topic)
                .map_err(|err| anyhow::format_err!("{:?}", err))?;
            state.broadcast.subscribe(topic);
        }
        debug!("Subscribing to topic {}", topic_name);
        Ok(receiver)
    }

    pub async fn stats(&self) -> Result<IpfsStats> {
        let (bs, swarm) = {
            let swarm = self.lock();
            let bs = swarm.block_store.clone();
            let state = swarm.discovery.state.clone();
            (bs, state)
        };
        let stats = bs.stats().await?;
        Ok(IpfsStats {
            repo_size: stats.size(),
            num_objects: stats.count(),
            swarm,
        })
    }
}

#[derive(Clone, Debug)]
pub struct IpfsStats {
    pub repo_size: u64,
    pub num_objects: u64,
    pub swarm: SwarmState,
}

#[cfg(test)]
mod tests {
    use super::*;
    use libipld::cbor::DagCborCodec;
    use libipld::codec::Codec;
    use libipld::multihash::{Code, MultihashDigest};
    use libipld::{ipld, Cid, Ipld};

    pub fn block_with_links(base: &str, links: Vec<Cid>) -> Block {
        let ipld = if links.is_empty() {
            ipld!(base)
        } else {
            ipld!({
                base: links.into_iter().map(Into::into).collect::<Vec<Ipld>>(),
            })
        };
        let data = DagCborCodec.encode(&ipld).unwrap();
        let hash = Code::Sha2_256.digest(&data);
        let cid = Cid::new_v1(DagCborCodec.into(), hash);
        Block::new_unchecked(cid, data)
    }

    pub fn cid(block: &Block) -> Cid {
        *block.cid()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn block_store() -> Result<()> {
        let ipfs_node = IpfsNode::test().await?;
        let b1_1a = block_with_links("b1_1a", Vec::new());
        let b1_1b = block_with_links("b1_1b", Vec::new());
        let b1_2 = block_with_links("b1_2", vec![cid(&b1_1a), cid(&b1_1b)]);
        ipfs_node.insert(b1_1a.clone()).await.unwrap();
        ipfs_node.insert(b1_1b.clone()).await.unwrap();
        ipfs_node.insert(b1_2.clone()).await.unwrap();
        assert_eq!(ipfs_node.fetch(b1_2.cid()).await.unwrap(), b1_2);
        assert_eq!(ipfs_node.fetch(b1_1a.cid()).await.unwrap(), b1_1a);
        assert_eq!(ipfs_node.fetch(b1_1b.cid()).await.unwrap(), b1_1b);
        Ok(())
    }
}
