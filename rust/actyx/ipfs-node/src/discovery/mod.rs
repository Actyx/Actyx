//! The purpose of this module is to add a method of peer discovery that does not use the
//! DHT.
//!
//! The traditional way of peer discovery in ipfs is to store the mapping from peer id
//! to the set of available addresses in the DHT, and then ask for that record via a DHT
//! query. New peer ids to discover are found by querying the DHT for content or even
//! for a random piece of data just to get some random closest peer ids.
//!
//! In our case, we have found that the DHT has quite some issues with lag, the need to
//! reprovide these records periodically, and the general complexity of it all. In
//! addition, I just could not get it to work quickly with the KAD behaviour.
//!
//! Fortunately, our typical installation is quite a bit different to the global ipfs.
//! We have a limited number of peers (<=1000 for the time being), so it is perfectly
//! OK to store a little bit of information for each peer.
//!
//! This module contains a state machine to keep track of a local view of the state
//! of a swarm that can be updated from different sources of information. It can track
//! the state of the node itself (which addresses it is listening on, and under which
//! addresses is it visible to other nodes), as well as the state of other nodes as
//! discovered via different mechanisms. The state machine itself is a completely
//! passive data object that needs to be explicitly updated.
//!
//! This module also contains a protocol definition for a pubsub protocol for peer
//! discovery. The idea is that every node regularly publishes what it knows about itself
//! via some sort of broadcast mechanism, and gathers information from other nodes
//! by listening in on the gossip of this broadcast mechanism. Fortunately, ipfs
//! already contains a built-in broadcast mechanism, gossipsub, which we have found
//! pretty reliable to get information to nodes even when they are not directly connected.
//!
//! The third part of this module is a libp2p behaviour that offers the gathered
//! information to the libp2p swarm object on request, and also triggers connection
//! attempts by emitting events for the libp2p swarm object. Using this behaviour,
//! as with all behaviours, requires a bit of manual wiring at the level of the libp2p
//! swarm object.
//!
//! Details about the individual components can be found in the module level documentation
//! of the components.
mod formats;
mod protocol;
mod state;
mod util;

use self::protocol::{DiscoveryMessage, ExpiredListenAddr, NewListenAddr, PublishMode};
pub use self::state::{AddressProvenance, SwarmState};
pub use self::util::strip_peer_id;
use libp2p::{
    core::connection::ConnectionId,
    gossipsub::{GossipsubEvent, IdentTopic, TopicHash},
    mdns::MdnsEvent,
    swarm::{
        protocols_handler::DummyProtocolsHandler, DialPeerCondition, NetworkBehaviour, NetworkBehaviourAction,
        PollParameters, ProtocolsHandler, SwarmEvent,
    },
    Multiaddr, PeerId,
};
use std::{
    collections::{BTreeSet, VecDeque},
    task::{Context, Poll},
    time::Duration,
};
use tracing::*;

/// discovery topic, hardcoded for now
const DISCOVERY_TOPIC: &str = "discovery";
/// discovery interval, hardcoded for now
const DISCOVERY_INTERVAL: Duration = Duration::from_secs(30);
/// after at least this period any disconnected addresses will be garbage collected, hardcoded for now
const PRUNE_ADDRESS_AFTER: Duration = Duration::from_secs(3 * 86_400); // 3 days

// Just because relevant iterators are private in libp2p-mdns crate...
#[derive(Debug)]
pub enum MyMdnsEvent {
    Discovered(Vec<(PeerId, Multiaddr)>),
    #[allow(dead_code)] // not really, but seems the compiler cannot properly figure it out
    Expired(Vec<(PeerId, Multiaddr)>),
}

#[derive(Debug)]
pub enum DiscoveryEvent {
    Publish { topic: IdentTopic, message: Vec<u8> },
}

pub struct Discovery {
    /// state machine that tracks the state
    pub(crate) state: SwarmState,
    /// Pending events to be emitted when polled.
    events: VecDeque<NetworkBehaviourAction<void::Void, DiscoveryEvent>>,
    /// topic of the discovery mechanism
    pub(crate) topic: IdentTopic,
    /// topic hash of the discovery mechanism, for quicker lookup
    topic_hash: TopicHash,
    /// stream for when to emit a gossip message
    gossip_stream: tokio::time::Interval,
    /// publish in binary format (cbor)
    publish_mode: PublishMode,
}

#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    topic: String,
    interval: Duration,
    publish_mode: PublishMode,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            topic: DISCOVERY_TOPIC.into(),
            interval: DISCOVERY_INTERVAL,
            publish_mode: PublishMode::Json,
        }
    }
}

impl Discovery {
    pub fn new_with_defaults(peer_id: PeerId) -> Self {
        Self::new(peer_id, DiscoveryConfig::default())
    }

    pub fn new(peer_id: PeerId, config: DiscoveryConfig) -> Self {
        let topic = IdentTopic::new(config.topic);
        // add a slight delay to the first discovery gossip message.
        // if we do it immediately, it is likely to get lost because we are not yet connected.
        let start = tokio::time::Instant::now() + Duration::from_secs(1);
        Self {
            state: SwarmState::new(peer_id),
            events: VecDeque::new(),
            topic_hash: topic.hash(),
            topic,
            gossip_stream: tokio::time::interval_at(start, config.interval),
            publish_mode: config.publish_mode,
        }
    }

    /// sift through swarm events and check if they contain something interesting for us
    pub fn add_swarm_event<T, E>(&mut self, event: &SwarmEvent<T, E>) {
        if !self.state.add_swarm_event(event) {
            // event did not result in a state change
            return;
        }
        match event {
            SwarmEvent::NewListenAddr(address) => {
                tracing::info!(target: "SWARM_SERVICES_BOUND", "Swarm Services bound to {}.", address);
                // immediately let other peers know that we have a new listen addr
                self.publish(DiscoveryMessage::NewListenAddr(NewListenAddr {
                    peer: *self.peer_id(),
                    addr: address.clone(),
                }));
            }
            SwarmEvent::ExpiredListenAddr(address) => {
                // immediately let other peers know that a listen addr has expired
                self.publish(DiscoveryMessage::ExpiredListenAddr(ExpiredListenAddr {
                    peer: *self.peer_id(),
                    addr: address.clone(),
                }));
            }
            _ => {}
        }
    }

    /// sift through gossipsub events and check if they contain something interesting for us
    pub fn add_gossipsub_event(&mut self, event: &GossipsubEvent) {
        trace!("add gossipsub event!");
        match event {
            GossipsubEvent::Message {
                propagation_source,
                message_id,
                message,
                ..
            } if message.topic == self.topic_hash => {
                if let Ok(msg) = DiscoveryMessage::from_bytes(&message.data) {
                    debug!(
                        "got relevant gossipsub message from:{} id:{} source:{:?}",
                        propagation_source, message_id, msg
                    );
                    let to_dial = self.state.add_discovery_message(msg);
                    self.dial_addresses(to_dial);
                }
            }
            // we might want to send out info when we have a new gossipsub peer...
            // GossipsubEvent::Subscribed { peer_id, topic } if *topic == self.topic_hash => {
            //     let info = self.state.own_node_info();
            //     debug!("publishing own addresses because we have a new listener {} {:?}", peer_id, info);
            //     self.publish(DiscoveryMessage::NodeInfo(info));
            // }
            _ => {
                // not for us
            }
        }
    }

    /// add an Mdns event. It is definitely interesting for us.
    pub fn add_mdns_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(iter) => {
                let v: Vec<(PeerId, Multiaddr)> = iter.collect();
                self.process_mdns_event(MyMdnsEvent::Discovered(v));
            }
            MdnsEvent::Expired(iter) => {
                let v: Vec<(PeerId, Multiaddr)> = iter.collect();
                self.process_mdns_event(MyMdnsEvent::Discovered(v));
            }
        }
    }

    pub(crate) fn process_mdns_event(&mut self, event: MyMdnsEvent) {
        match event {
            MyMdnsEvent::Discovered(v) => {
                #[allow(clippy::mutable_key_type)] // clippy bug #5812
                let mut to_dial: BTreeSet<PeerId> = BTreeSet::new();
                for (peer_id, address) in v.into_iter() {
                    debug!(
                        "discovered new listen addr via MDns: peer:{} address:{}",
                        peer_id, address
                    );
                    if !self
                        .state
                        .add_listen_addr(peer_id, address, AddressProvenance::MDNS)
                        .is_empty()
                    {
                        // immediately dial newly discovered peers
                        to_dial.insert(peer_id);
                    }
                }
                for peer in to_dial.into_iter() {
                    self.dial_peer(peer);
                }
            }
            MyMdnsEvent::Expired(v) => {
                for (peer_id, address) in v.into_iter() {
                    debug!("listen address expired from MDns: peer:{} address:{}", peer_id, address);
                    self.state.check_and_prune_expired_mdns_address(&peer_id, &address);
                }
            }
        }
    }

    /// periodically called to trigger gossip
    fn gossip_node_info(&mut self) {
        // debug!("connected peers: {:?}", self.state.connected_peers());

        let info = self.state.own_node_info();
        debug!("publishing own addresses periodically {:?}", info);
        self.publish(DiscoveryMessage::NodeInfo(info));
    }

    /// Periodically called to dial disconnected peers.
    /// Will GC peers that have been disconnected for too long
    /// and none of their addresses originated via bootstrap.
    pub(crate) fn prune_and_dial_disconnected_peers(&mut self) {
        // just try periodically to dial all disconnected peers
        // first GC the peers that have been disconnected for too long
        self.state.gc_expired_addresses_and_peers();
        // then dial all remaining disconnected peers
        for peer in self.state.disconnected_peers() {
            self.dial_peer(peer);
        }
    }

    fn dial_addresses(&mut self, addresses: impl IntoIterator<Item = Multiaddr>) {
        let addresses: BTreeSet<Multiaddr> = addresses.into_iter().collect();
        if addresses.is_empty() {
            return;
        }
        debug!("telling swarm to dial addresses {:?}", addresses);
        for address in addresses {
            self.events.push_back(NetworkBehaviourAction::DialAddress { address })
        }
    }

    fn dial_peer(&mut self, peer_id: PeerId) {
        debug!("telling swarm to dial peer {:?}", peer_id);
        self.events.push_back(NetworkBehaviourAction::DialPeer {
            peer_id,
            condition: DialPeerCondition::Disconnected,
        });
    }

    fn peer_id(&self) -> &PeerId {
        &self.state.peer_id()
    }

    fn publish(&mut self, message: DiscoveryMessage) {
        self.events
            .push_back(NetworkBehaviourAction::GenerateEvent(DiscoveryEvent::Publish {
                topic: self.topic.clone(),
                message: message.to_bytes(self.publish_mode),
            }));
    }
}

impl NetworkBehaviour for Discovery {
    type ProtocolsHandler = DummyProtocolsHandler;
    type OutEvent = DiscoveryEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    /// this is being called by libp2p if it wants to know the addresses of a peer
    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        self.state.addresses_of_peer(peer_id).into_iter().collect()
    }

    fn inject_connected(&mut self, _: &PeerId) {}

    fn inject_disconnected(&mut self, _: &PeerId) {}

    fn inject_event(&mut self, _: PeerId, _: ConnectionId, ev: <Self::ProtocolsHandler as ProtocolsHandler>::OutEvent) {
        void::unreachable(ev);
    }

    fn poll(
        &mut self,
        context: &mut Context,
        poll_parameters: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<<Self::ProtocolsHandler as ProtocolsHandler>::InEvent, Self::OutEvent>> {
        self.state.apply_poll_parameters(poll_parameters);
        if self.gossip_stream.poll_tick(context).is_ready() {
            self.gossip_node_info();
            self.prune_and_dial_disconnected_peers();
        }
        if let Some(event) = self.events.pop_front() {
            Poll::Ready(event)
        } else {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::mutable_key_type)] // clippy bug #5812
    #![allow(clippy::redundant_clone)]
    use super::protocol::*;
    use super::*;
    use futures_test::task::noop_context;
    use libp2p::core::ConnectedPoint;
    use libp2p::gossipsub::{GossipsubMessage, MessageId};
    use libp2p::multiaddr::{Multiaddr, Protocol};
    use libp2p::swarm::{AddressRecord, NetworkBehaviour};
    use maplit::*;
    use rand::Rng;
    use std::collections::{BTreeSet, HashSet};
    use std::{net::Ipv6Addr, str::FromStr};

    #[derive(Debug, Clone, Hash, PartialEq, Eq)]
    enum SwarmAction {
        DialAddress(Multiaddr),
        DialPeer(PeerId),
    }

    // call poll on a bitswap with dummy params
    fn poll(discovery: &mut Discovery, ctx: &mut Context) -> Poll<NetworkBehaviourAction<void::Void, DiscoveryEvent>> {
        let mut params = DummyPollParameters;
        discovery.poll(ctx, &mut params)
    }

    /// call to drain the behaviour until it returns pending.
    fn poll_until_pending(discovery: &mut Discovery) -> (Vec<DiscoveryMessage>, HashSet<SwarmAction>) {
        let mut ctx = noop_context();
        let mut messages: Vec<DiscoveryMessage> = Default::default();
        let mut actions: HashSet<SwarmAction> = Default::default();
        while let Poll::Ready(ev) = poll(discovery, &mut ctx) {
            match ev {
                NetworkBehaviourAction::GenerateEvent(e) => match e {
                    DiscoveryEvent::Publish { topic, message } => {
                        assert_eq!(&topic.to_string(), DISCOVERY_TOPIC);
                        let msg = super::protocol::DiscoveryMessage::from_bytes(&message).unwrap();
                        messages.push(msg);
                    }
                },
                NetworkBehaviourAction::DialAddress { address } => {
                    actions.insert(SwarmAction::DialAddress(address));
                }
                NetworkBehaviourAction::DialPeer { peer_id, condition: _ } => {
                    actions.insert(SwarmAction::DialPeer(peer_id));
                }
                ev => {
                    panic!("Unexpected NetworkBehaviourAction from Bitswap: {:?}", ev);
                }
            };
        }
        (messages, actions)
    }

    struct DummyPollParameters;
    impl PollParameters for DummyPollParameters {
        type SupportedProtocolsIter = std::iter::Empty<Vec<u8>>;
        type ListenedAddressesIter = std::iter::Empty<Multiaddr>;
        type ExternalAddressesIter = std::iter::Empty<AddressRecord>;

        fn supported_protocols(&self) -> Self::SupportedProtocolsIter {
            std::iter::empty()
        }

        fn listened_addresses(&self) -> Self::ListenedAddressesIter {
            std::iter::empty()
        }

        fn external_addresses(&self) -> Self::ExternalAddressesIter {
            std::iter::empty()
        }

        fn local_peer_id(&self) -> &PeerId {
            unimplemented!()
        }
    }

    fn peer() -> PeerId {
        PeerId::random()
    }

    fn multiaddr() -> Multiaddr {
        let mut rng = rand::thread_rng();
        let mut data = [0u8; 16];
        rng.fill(&mut data);
        let mut addr = Multiaddr::empty();
        addr.push(Protocol::Ip6(Ipv6Addr::from(data)));
        addr.push(Protocol::Tcp(rng.gen()));
        addr
    }

    fn node_info(peer: PeerId, addrs: BTreeSet<Multiaddr>) -> NodeInfo {
        NodeInfo {
            stats: Default::default(),
            addresses: btreemap! { peer => addrs },
        }
    }

    fn inject_swarm_event(discovery: &mut Discovery, event: &SwarmEvent<(), ()>) {
        discovery.add_swarm_event(event);
    }

    fn inject_gossipsub_message<T: Into<DiscoveryMessage>>(discovery: &mut Discovery, msg: T) {
        let peer = peer();
        let msg: DiscoveryMessage = msg.into();
        discovery.add_gossipsub_event(&GossipsubEvent::Message {
            propagation_source: peer,
            message_id: MessageId::new(b"0"),
            message: GossipsubMessage {
                source: Some(peer),
                data: msg.to_bytes(PublishMode::Json),
                sequence_number: Some(1234),
                topic: IdentTopic::new(DISCOVERY_TOPIC).hash(),
            },
        })
    }

    /// check that we properly track and publish our own listen addrs
    #[tokio::test]
    async fn new_expired_listen_addr() {
        tokio::time::pause();
        let me = peer();
        let mut discovery = Discovery::new(me, DiscoveryConfig::default());
        // simulate new listen addr for ourselves
        let a1 = Multiaddr::from_str("/ip4/127.0.0.1/tcp/4001").unwrap();
        let a2 = Multiaddr::from_str("/ip4/8.8.8.8/tcp/4001").unwrap();

        // we discovered a1
        inject_swarm_event(&mut discovery, &SwarmEvent::NewListenAddr(a1.clone()));
        let (m, _a) = poll_until_pending(&mut discovery);
        // it should immediately send out this info via pubsub
        assert_eq!(
            m,
            vec![DiscoveryMessage::NewListenAddr(NewListenAddr {
                peer: me,
                addr: a1.clone()
            })]
        );
        // it should incorporate the new info
        assert_eq!(discovery.state.listen_addrs(), &btreeset! {a1.clone()});

        // we discovered a1 again
        inject_swarm_event(&mut discovery, &SwarmEvent::NewListenAddr(a1.clone()));
        let (m, _a) = poll_until_pending(&mut discovery);
        // it should not send out this info via pubsub, since we already knew it
        assert_eq!(m, vec![]);

        // we discovered a2
        inject_swarm_event(&mut discovery, &SwarmEvent::NewListenAddr(a2.clone()));
        let (m, _a) = poll_until_pending(&mut discovery);
        // it should immediately send out this info via pubsub
        assert_eq!(
            m,
            vec![DiscoveryMessage::NewListenAddr(NewListenAddr {
                peer: me,
                addr: a2.clone()
            })]
        );
        // it should incorporate the new info
        assert_eq!(discovery.state.listen_addrs(), &btreeset! {a1.clone(), a2.clone()});

        // a1 is expired
        inject_swarm_event(&mut discovery, &SwarmEvent::ExpiredListenAddr(a1.clone()));
        let (m, _a) = poll_until_pending(&mut discovery);
        // it should immediately send out this info via pubsub
        assert_eq!(
            m,
            vec![DiscoveryMessage::ExpiredListenAddr(ExpiredListenAddr {
                peer: me,
                addr: a1.clone()
            })]
        );
        // it should incorporate the new info
        assert_eq!(discovery.state.listen_addrs(), &btreeset! {a2.clone()});

        inject_swarm_event(&mut discovery, &SwarmEvent::ExpiredListenAddr(a2.clone()));
        let (m, _a) = poll_until_pending(&mut discovery);
        assert_eq!(
            m,
            vec![DiscoveryMessage::ExpiredListenAddr(ExpiredListenAddr {
                peer: me,
                addr: a2.clone()
            })]
        );
        // it should incorporate the new info
        assert_eq!(discovery.state.listen_addrs(), &btreeset! {});
    }

    /// check that we dial new nodes as we get info about them
    #[tokio::test]
    async fn dial_new_nodes() {
        tokio::time::pause();
        let me = peer();
        let p1 = peer();
        let a1 = multiaddr();
        let p2 = peer();
        let a2 = multiaddr();
        let mut discovery = Discovery::new(me, DiscoveryConfig::default());

        // we learn of a new peer via pubsub NewListenAddr
        inject_gossipsub_message(
            &mut discovery,
            NewListenAddr {
                peer: p1,
                addr: a1.clone(),
            },
        );
        let (_, a) = poll_until_pending(&mut discovery);
        // check that we are immediately dialing this peer, since we are not connected to it
        assert_eq!(a, hashset! { SwarmAction::DialAddress(a1.clone()) });
        // check that we incorporate this information
        assert_eq!(discovery.state.addresses_of_peer(&p1), btreeset! {a1.clone()});

        // we learn of a new peer via pubsub NodeInfo
        inject_gossipsub_message(&mut discovery, node_info(p2, btreeset! { a2.clone() }));
        let (_, a) = poll_until_pending(&mut discovery);
        // check that we are immediately dialing this peer, since we are not connected to it
        assert_eq!(a, hashset! { SwarmAction::DialAddress(a2.clone()) });
        // check that we incorporate this information
        assert_eq!(discovery.state.addresses_of_peer(&p2), btreeset! {a2.clone()});

        // we have learned about the address via pubsub (swarm interaction), let's see if this is reflected
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p2),
            vec!((a2.clone(), AddressProvenance::Swarm))
        );

        // trigger the publishing of our own node info and dialing of disconnected peers
        tokio::time::advance(DISCOVERY_INTERVAL).await;
        let (e, a) = poll_until_pending(&mut discovery);
        // check that we tried to dial all disconnected peers
        assert_eq!(a, hashset! { SwarmAction::DialPeer(p1), SwarmAction::DialPeer(p2) });
        // check that we sent out our own node info
        if let Some(DiscoveryMessage::NodeInfo(info)) = e.get(0) {
            assert_eq!(
                info,
                &NodeInfo {
                    stats: NodeStats {
                        known_peers: 2,
                        connected_peers: 0
                    },
                    addresses: btreemap! { me => btreeset!{} }
                }
            )
        } else {
            panic!()
        }
    }

    /// check that we dial the bootstrap nodes
    #[tokio::test]
    async fn dial_bootstrap() {
        tokio::time::pause();
        let me = peer();
        let p1 = peer();
        let mut a1 = multiaddr();
        a1.push(Protocol::P2p(p1.clone().into()));
        let p2 = peer();
        let mut a2 = multiaddr();
        let a2c = a2.clone();
        a2.push(Protocol::P2p(p2.clone().into()));
        let mut discovery = Discovery::new(me, DiscoveryConfig::default());
        discovery.state.add_bootstrap(a1);
        discovery.state.add_bootstrap(a2);
        // we have learned about the address via bootstrap, let's see if this is reflected
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p2),
            vec!((a2c, AddressProvenance::Bootstrap))
        );
        discovery.prune_and_dial_disconnected_peers();
        let (_, a) = poll_until_pending(&mut discovery);
        assert_eq!(a, hashset! { SwarmAction::DialPeer(p1), SwarmAction::DialPeer(p2) });
    }

    fn disconnect(discovery: &mut Discovery, p: &PeerId, a: &Multiaddr) -> bool {
        discovery
            .state
            .add_swarm_event::<(), ()>(&SwarmEvent::ConnectionClosed {
                peer_id: *p,
                endpoint: ConnectedPoint::Dialer { address: a.clone() },
                num_established: 0,
                cause: Some(libp2p::core::connection::ConnectionError::IO(std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset,
                    "sorry!",
                ))),
            })
    }

    fn cannonicalise_vec(v: &[(Multiaddr, AddressProvenance)]) -> BTreeSet<(String, AddressProvenance)> {
        let v1: BTreeSet<(String, AddressProvenance)> = v.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        v1
    }

    #[tokio::test]
    async fn check_gc_acquired_addresses() {
        tokio::time::pause();
        let me = peer();
        let p1 = peer();
        let a1 = multiaddr();
        let p2 = peer();
        let a2 = multiaddr();
        let mut a3 = multiaddr();
        let a3c = a3.clone();
        a3.push(Protocol::P2p(p1.clone().into()));
        let mut discovery = Discovery::new(me, DiscoveryConfig::default());

        // we learn of a new peer via pubsub NewListenAddr
        inject_gossipsub_message(
            &mut discovery,
            NewListenAddr {
                peer: p1,
                addr: a1.clone(),
            },
        );
        let (_, a) = poll_until_pending(&mut discovery);
        // check that we are immediately dialing this peer, since we are not connected to it
        assert_eq!(a, hashset! { SwarmAction::DialAddress(a1.clone()) });
        // check that we incorporate this information
        assert_eq!(discovery.state.addresses_of_peer(&p1), btreeset! {a1.clone()});

        // now we inject another address via bootstrap
        discovery.state.add_bootstrap(a3);
        // check that we incorporate this information
        assert_eq!(
            discovery.state.addresses_of_peer(&p1),
            btreeset! {a1.clone(), a3c.clone()}
        );

        // we learn of a new peer via pubsub NodeInfo
        inject_gossipsub_message(&mut discovery, node_info(p2, btreeset! { a2.clone() }));
        let (_, a) = poll_until_pending(&mut discovery);
        // check that we are immediately dialing this peer, since we are not connected to it
        assert_eq!(a, hashset! { SwarmAction::DialAddress(a2.clone()) });
        // check that we incorporate this information
        assert_eq!(discovery.state.addresses_of_peer(&p2), btreeset! {a2.clone()});

        // we have learned about the address via discovery and bootstrap, let's see if this is reflected
        let v1 = cannonicalise_vec(&discovery.state.addresses_of_peer_with_provenance(&p1));
        let v2 = cannonicalise_vec(&[
            (a1.clone(), AddressProvenance::Discovery),
            (a3c.clone(), AddressProvenance::Bootstrap),
        ]);
        assert_eq!(v1, v2);
        // we have learned about the address via pubsub (swarm interaction), let's see if this is reflected
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p2),
            vec!((a2.clone(), AddressProvenance::Swarm))
        );

        // Disconnect on all addresses of p1, including a3 - the bootstrap address
        disconnect(&mut discovery, &p1, &a1);
        disconnect(&mut discovery, &p1, &a3c);

        tokio::time::advance(PRUNE_ADDRESS_AFTER + Duration::from_secs(86_400)).await; // one day plus PRUNE_ADDRESS_AFTER has passed...

        discovery.prune_and_dial_disconnected_peers();

        // one discovered disconnected address has been gc'ed, while the bootstrapped one survived
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p1),
            vec!((a3c.clone(), AddressProvenance::Bootstrap))
        );
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p2),
            vec!((a2.clone(), AddressProvenance::Swarm))
        );
    }

    #[tokio::test]
    async fn check_gc_and_proper_provenance() {
        tokio::time::pause();
        let me = peer();
        let p1 = peer();
        let a1 = multiaddr();
        let p2 = peer();
        let a2 = multiaddr();
        let mut a3 = multiaddr();
        let a3c = a3.clone();
        a3.push(Protocol::P2p(p1.clone().into()));
        let mut discovery = Discovery::new(me, DiscoveryConfig::default());

        // we learn of a new peer via pubsub NewListenAddr
        inject_gossipsub_message(
            &mut discovery,
            NewListenAddr {
                peer: p1,
                addr: a1.clone(),
            },
        );
        let (_, a) = poll_until_pending(&mut discovery);
        // check that we are immediately dialing this peer, since we are not connected to it
        assert_eq!(a, hashset! { SwarmAction::DialAddress(a1.clone()) });
        // check that we incorporate this information
        assert_eq!(discovery.state.addresses_of_peer(&p1), btreeset! {a1.clone()});

        // we learn of a new peer via pubsub NewListenAddr
        inject_gossipsub_message(
            &mut discovery,
            NewListenAddr {
                peer: p1,
                addr: a3.clone(),
            },
        );
        let (_, a) = poll_until_pending(&mut discovery);
        // check that we are immediately dialing this peer, since we are not connected to it
        assert_eq!(a, hashset! { SwarmAction::DialAddress(a3c.clone()) });

        // check that we incorporate this information
        assert_eq!(
            discovery.state.addresses_of_peer(&p1),
            btreeset! {a1.clone(), a3c.clone()}
        );

        // we have learned about the addresses via discovery, let's see if this is reflected
        let v1 = cannonicalise_vec(&discovery.state.addresses_of_peer_with_provenance(&p1));
        let v2 = cannonicalise_vec(&[
            (a1.clone(), AddressProvenance::Discovery),
            (a3c.clone(), AddressProvenance::Discovery),
        ]);
        assert_eq!(v1, v2);

        // now we learn about a3 via bootstrap
        discovery.state.add_bootstrap(a3.clone());

        // let's see if this is reflected properly
        let v1 = cannonicalise_vec(&discovery.state.addresses_of_peer_with_provenance(&p1));
        let v2 = cannonicalise_vec(&[
            (a1.clone(), AddressProvenance::Discovery),
            (a3c.clone(), AddressProvenance::Bootstrap),
        ]);
        assert_eq!(v1, v2);

        // we learn of a3 again via discovery
        inject_gossipsub_message(
            &mut discovery,
            NewListenAddr {
                peer: p1,
                addr: a3.clone(),
            },
        );
        let (_, _a) = poll_until_pending(&mut discovery);

        // let's see that a3 has not been "downgraded"
        let v1 = cannonicalise_vec(&discovery.state.addresses_of_peer_with_provenance(&p1));
        let v2 = cannonicalise_vec(&[
            (a1.clone(), AddressProvenance::Discovery),
            (a3c.clone(), AddressProvenance::Bootstrap),
        ]);
        assert_eq!(v1, v2);

        // we learn of a new peer via pubsub NodeInfo
        inject_gossipsub_message(&mut discovery, node_info(p2, btreeset! { a2.clone() }));
        let (_, a) = poll_until_pending(&mut discovery);
        // check that we are immediately dialing this peer, since we are not connected to it
        assert_eq!(a, hashset! { SwarmAction::DialAddress(a2.clone()) });
        // check that we incorporate this information
        assert_eq!(discovery.state.addresses_of_peer(&p2), btreeset! {a2.clone()});

        // we have learned about the address via pubsub (swarm interaction), let's see if this is reflected
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p2),
            vec!((a2.clone(), AddressProvenance::Swarm))
        );

        // now we learn about this address via discovery
        // we learn of a new peer via pubsub NewListenAddr
        inject_gossipsub_message(
            &mut discovery,
            NewListenAddr {
                peer: p2,
                addr: a2.clone(),
            },
        );
        let (_, _a) = poll_until_pending(&mut discovery);

        // let's see if this is reflected
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p2),
            vec!((a2.clone(), AddressProvenance::Discovery))
        );

        // Disconnect on all addresses of p1, including a3 - the bootstrap address
        disconnect(&mut discovery, &p1, &a1);
        disconnect(&mut discovery, &p1, &a3c);

        tokio::time::advance(PRUNE_ADDRESS_AFTER + Duration::from_secs(86_400)).await; // one day plus PRUNE_ADDRESS_AFTER has passed...

        discovery.prune_and_dial_disconnected_peers();

        // one discovered disconnected address has been gc'ed, while the bootstrapped one survived
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p1),
            vec!((a3c.clone(), AddressProvenance::Bootstrap))
        );
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p2),
            vec!((a2.clone(), AddressProvenance::Discovery))
        );

        // now let's learn about a4 via MDNS
        let a4 = multiaddr();
        discovery.process_mdns_event(MyMdnsEvent::Discovered(vec![(p1, a4.clone())]));
        let _ = poll_until_pending(&mut discovery);
        // check that we incorporate this information
        let v1 = cannonicalise_vec(&discovery.state.addresses_of_peer_with_provenance(&p1));
        let v2 = cannonicalise_vec(&[
            (a3c.clone(), AddressProvenance::Bootstrap),
            (a4.clone(), AddressProvenance::MDNS),
        ]);
        assert_eq!(v1, v2);

        // and now we learn that this address has expired...
        discovery.process_mdns_event(MyMdnsEvent::Expired(vec![(p1, a4.clone())]));

        // and ... ?
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p1),
            vec!((a3c.clone(), AddressProvenance::Bootstrap))
        );
    }

    #[tokio::test]
    async fn check_no_provenance_regressions() {
        tokio::time::pause();
        let me = peer();
        let p1 = peer();
        let mut a1 = multiaddr();
        let a1c = a1.clone();
        a1.push(Protocol::P2p(p1.clone().into()));
        let mut discovery = Discovery::new(me, DiscoveryConfig::default());

        // we learn of a new peer via pubsub NodeInfo
        inject_gossipsub_message(&mut discovery, node_info(p1, btreeset! { a1.clone() }));
        let (_, a) = poll_until_pending(&mut discovery);
        // check that we are immediately dialing this peer, since we are not connected to it
        assert_eq!(a, hashset! { SwarmAction::DialAddress(a1c.clone()) });
        // check that we incorporate this information
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p1),
            vec!((a1c.clone(), AddressProvenance::Swarm))
        );

        discovery.process_mdns_event(MyMdnsEvent::Discovered(vec![(p1, a1.clone())]));
        let _ = poll_until_pending(&mut discovery);
        // check that we incorporate this information
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p1),
            vec!((a1c.clone(), AddressProvenance::MDNS))
        );

        // now we learn about this address via discovery
        // we learn of a new peer via pubsub NewListenAddr
        inject_gossipsub_message(
            &mut discovery,
            NewListenAddr {
                peer: p1,
                addr: a1.clone(),
            },
        );
        let (_, _a) = poll_until_pending(&mut discovery);

        // let's see if this is reflected
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p1),
            vec!((a1c.clone(), AddressProvenance::Discovery))
        );

        // let's see if we don't regress from Discovery to MDNS
        discovery.process_mdns_event(MyMdnsEvent::Discovered(vec![(p1, a1.clone())]));
        let _ = poll_until_pending(&mut discovery);
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p1),
            vec!((a1c.clone(), AddressProvenance::Discovery))
        );

        // now let's learn about a1 via bootstrap
        discovery.state.add_bootstrap(a1.clone());
        // check that we incorporate this information
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p1),
            vec!((a1c.clone(), AddressProvenance::Bootstrap))
        );

        // let's see if we don't regress from Bootstrap to MDNS
        discovery.process_mdns_event(MyMdnsEvent::Discovered(vec![(p1, a1.clone())]));
        let _ = poll_until_pending(&mut discovery);
        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p1),
            vec!((a1c.clone(), AddressProvenance::Bootstrap))
        );

        // let's see if we don't regress from Bootstrap to Discovery
        inject_gossipsub_message(
            &mut discovery,
            NewListenAddr {
                peer: p1,
                addr: a1.clone(),
            },
        );
        let (_, _a) = poll_until_pending(&mut discovery);

        assert_eq!(
            discovery.state.addresses_of_peer_with_provenance(&p1),
            vec!((a1c.clone(), AddressProvenance::Bootstrap))
        );
    }
}
