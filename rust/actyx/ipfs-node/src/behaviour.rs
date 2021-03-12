use crate::{
    bitswap::{Bitswap, BitswapEvent},
    block_store::{BlockAdapter, BlockStore},
    discovery::{Discovery, DiscoveryEvent},
    sync::SyncStates,
};
use ax_futures_util::{future::OneShotDispatcher, stream::StreamDispatcher};
use fnv::FnvHashSet;
use futures::{
    channel::mpsc,
    prelude::*,
    task::{Context, Poll},
};
use libipld::Cid;
use libp2p::{
    gossipsub::{Gossipsub, GossipsubConfig, GossipsubEvent, IdentTopic, MessageAuthenticity},
    identify::{Identify, IdentifyEvent},
    identity::Keypair,
    mdns::{Mdns, MdnsEvent},
    ping::PingConfig,
    ping::{self, Ping, PingEvent},
    swarm::{
        toggle::Toggle, IntoProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess,
        PollParameters, ProtocolsHandler, SwarmEvent,
    },
    NetworkBehaviour, PeerId,
};
use libp2p_ax_bitswap::Block;
use libp2p_broadcast::{BroadcastBehaviour, BroadcastEvent, Topic};
use std::collections::{BTreeMap, BTreeSet};
use tracing::*;

pub enum StoreResponse {
    /// Send an answer to a block request over bitswap
    BlockSend { peer: PeerId, blocks: BTreeSet<Block> },

    /// Send an answer to a have request over bitswap
    BlockHave { peer: PeerId, info: BTreeMap<Cid, bool> },
}

type THandler<T> = <<T as NetworkBehaviour>::ProtocolsHandler as IntoProtocolsHandler>::Handler;

/// a minimal network behaviour that is still treated as a full node by other ipfs peers
#[derive(NetworkBehaviour)]
#[behaviour(poll_method = "poll", out_event = "()")]
pub struct Behaviour {
    pub bitswap: Bitswap,
    pub gossipsub: Gossipsub,
    pub(crate) discovery: Discovery,
    pub(crate) identify: Identify,
    pub(crate) ping: Ping,
    pub(crate) mdns: Toggle<Mdns>,
    pub(crate) broadcast: BroadcastBehaviour,

    #[behaviour(ignore)]
    store_sender: mpsc::UnboundedSender<StoreResponse>,

    #[behaviour(ignore)]
    store_receiver: mpsc::UnboundedReceiver<StoreResponse>,

    #[behaviour(ignore)]
    pub(crate) block_store: BlockStore,

    #[behaviour(ignore)]
    pub(crate) block_listeners: OneShotDispatcher<Cid, Block>,

    #[behaviour(ignore)]
    pub(crate) topic_listeners: StreamDispatcher<Topic, Vec<u8>>,

    #[behaviour(ignore)]
    pub(crate) sync_states: SyncStates,
    #[behaviour(ignore)]
    pub(crate) allow_publish: bool,
}

impl Behaviour {
    pub async fn new(
        keypair: Keypair,
        gossipsub_config: GossipsubConfig,
        ping_config: PingConfig,
        block_store: BlockStore,
        use_mdns: bool,
        allow_publish: bool,
    ) -> anyhow::Result<Self> {
        let public_key = keypair.public();
        let local_peer_id = public_key.clone().into_peer_id();
        let (store_sender, store_receiver) = futures::channel::mpsc::unbounded::<StoreResponse>();
        let mut gossipsub = Gossipsub::new(MessageAuthenticity::Signed(keypair), gossipsub_config)
            .map_err(|err| anyhow::format_err!("{}", err))?;
        let mdns = if use_mdns { Some(Mdns::new().await?) } else { None }.into();
        let discovery = Discovery::new_with_defaults(local_peer_id);
        gossipsub
            .subscribe(&discovery.topic)
            .map_err(|err| anyhow::format_err!("{:?}", err))?;

        Ok(Self {
            gossipsub,
            identify: Identify::new("ipfs/1.0.0".into(), "actyx".into(), public_key),
            ping: Ping::new(ping_config),
            bitswap: Bitswap::new(),
            discovery,
            mdns,
            broadcast: BroadcastBehaviour::default(),
            block_store,
            block_listeners: OneShotDispatcher::new(),
            topic_listeners: StreamDispatcher::new(),
            store_sender,
            store_receiver,
            sync_states: SyncStates::new(),
            allow_publish,
        })
    }

    fn poll(
        &mut self,
        ctx: &mut Context,
        _p: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<<THandler<Self> as ProtocolsHandler>::InEvent, ()>> {
        while let Some(topic) = self.topic_listeners.next() {
            self.broadcast.unsubscribe(&topic);
            let topic = IdentTopic::new(std::str::from_utf8(&topic).expect("is valid utf8"));
            if let Err(err) = self.gossipsub.unsubscribe(&topic) {
                tracing::error!("failed to unsubscribe from {}: {:?}", topic, err);
            }
        }
        while let Poll::Ready(Some(response)) = self.store_receiver.poll_next_unpin(ctx) {
            match response {
                StoreResponse::BlockHave { peer, info } => self.bitswap.send_haves(peer, info),
                StoreResponse::BlockSend { peer, blocks } => self.bitswap.send_blocks(peer, blocks),
            }
        }
        Poll::Pending
    }

    pub(crate) fn process_swarm_event(&mut self, ev: SwarmEvent<(), <THandler<Self> as ProtocolsHandler>::Error>) {
        self.discovery.add_swarm_event(&ev);
    }
}

impl NetworkBehaviourEventProcess<IdentifyEvent> for Behaviour {
    // Called when `identify` produces an event.
    fn inject_event(&mut self, event: IdentifyEvent) {
        trace!("{:?}", event);
    }
}

impl NetworkBehaviourEventProcess<PingEvent> for Behaviour {
    // Called when `ping` produces an event.
    #[allow(clippy::cognitive_complexity)]
    fn inject_event(&mut self, event: PingEvent) {
        use ping::handler::{PingFailure, PingSuccess};
        match event {
            PingEvent {
                peer,
                result: Result::Ok(PingSuccess::Ping { rtt }),
            } => {
                trace!("ping: rtt to {} is {} ms", peer.to_base58(), rtt.as_millis());
            }
            PingEvent {
                peer,
                result: Result::Ok(PingSuccess::Pong),
            } => {
                trace!("ping: pong from {}", peer.to_base58());
            }
            PingEvent {
                peer,
                result: Result::Err(PingFailure::Timeout),
            } => {
                trace!("ping: timeout to {}", peer.to_base58());
            }
            PingEvent {
                peer,
                result: Result::Err(PingFailure::Other { error }),
            } => {
                trace!("ping: failure with {}: {}", peer.to_base58(), error);
            }
        }
    }
}

impl NetworkBehaviourEventProcess<GossipsubEvent> for Behaviour {
    // Called when `gossipsub` produces an event.
    fn inject_event(&mut self, event: GossipsubEvent) {
        // give discovery a chance to inspect the event
        self.discovery.add_gossipsub_event(&event);
        match event {
            GossipsubEvent::Message { message, .. } => {
                // dispatch it to whoever is interested
                self.topic_listeners
                    .notify(Topic::new(message.topic.as_str().as_bytes()), message.data);
            }
            GossipsubEvent::Subscribed { peer_id, topic } => {
                // use this for discovery?
                debug!("GossipsubEvent::Subscribed({}, {:?})", peer_id, topic);
            }
            GossipsubEvent::Unsubscribed { peer_id, topic } => {
                // use this for discovery?
                debug!("GossipsubEvent::Unsubscribed({}, {:?})", peer_id, topic);
            }
        }
    }
}

impl NetworkBehaviourEventProcess<BroadcastEvent> for Behaviour {
    fn inject_event(&mut self, event: BroadcastEvent) {
        match event {
            BroadcastEvent::Received(_peer_id, topic, data) => {
                self.topic_listeners.notify(topic, data.to_vec());
            }
            BroadcastEvent::Subscribed(_, _) => {}
            BroadcastEvent::Unsubscribed(_, _) => {}
        }
    }
}

impl NetworkBehaviourEventProcess<DiscoveryEvent> for Behaviour {
    // Called when `discovery` produces an event.
    fn inject_event(&mut self, event: DiscoveryEvent) {
        match event {
            DiscoveryEvent::Publish { topic, message } => {
                if let Err(e) = self.gossipsub.publish(topic, message) {
                    if let libp2p::gossipsub::error::PublishError::InsufficientPeers = e {
                        debug!("Error publishing to gossipsub: {:?}", e);
                    } else {
                        error!("Error publishing to gossipsub: {:?}", e);
                    }
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<BitswapEvent> for Behaviour {
    // Called when `bitswap` produces an event.
    fn inject_event(&mut self, event: BitswapEvent) {
        match event {
            BitswapEvent::BlockHave { peer, cids, .. } => {
                let sender = self.store_sender.clone();
                tokio::spawn(self.block_store.clone().check_blocks(cids).map(move |res| {
                    match res {
                        Ok(info) => sender.unbounded_send(StoreResponse::BlockHave { peer, info }).unwrap(),
                        Err(cause) => error!("store error {}", cause),
                    };
                }));
            }
            BitswapEvent::BlockWanted { peer, cids, .. } => {
                let sender = self.store_sender.clone();
                tokio::spawn(self.block_store.clone().get_blocks(cids).map(move |res| {
                    match res {
                        Ok(blocks) => sender
                            .unbounded_send(StoreResponse::BlockSend { peer, blocks })
                            .unwrap(),
                        Err(cause) => error!("store error {}", cause),
                    };
                }));
            }
            BitswapEvent::BlocksReceived(blocks) => {
                // dispatch it to whoever is interested
                for block in blocks.iter() {
                    self.block_listeners.notify(*block.cid(), block.clone());
                }

                // store it.
                //
                // Since the write is done asynchronously, there is actually a period where the
                // block store could say that it doesn't have the block yet, even though we just
                // received it here, and we might end up sending out a new `want` request for
                // the same block.
                info!(
                    "adding blocks to store {:?}",
                    blocks.iter().map(|x| x.cid().to_string()).collect::<Vec<_>>().join(",")
                );
                let _ = self
                    .block_store
                    .inner()
                    .lock()
                    .put_blocks(blocks.into_iter().map(BlockAdapter), None);
                // tokio::spawn(self.block_store.put_blocks(blocks));
            }
            BitswapEvent::WantCleanup => {
                // remove all senders for which the receiver side has been dropped
                self.block_listeners.gc();
                // create set of cids that are still wanted after the gc
                let mut wanted: FnvHashSet<Cid> = self.block_listeners.keys().cloned().collect();
                // add all cids that are currently being synced. we don't want them to be cancelled.
                wanted.extend(self.sync_states.cids());
                // tell bitswap to clean up the wantlist
                self.bitswap.want_cleanup(|cid| wanted.contains(cid));
            }
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {
    // Called when `mdns` produces an event.
    fn inject_event(&mut self, event: MdnsEvent) {
        self.discovery.add_mdns_event(event);
    }
}
