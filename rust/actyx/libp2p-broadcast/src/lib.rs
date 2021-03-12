use crate::protocol::Message;
use fnv::{FnvHashMap, FnvHashSet};
use libp2p::core::connection::ConnectionId;
use libp2p::swarm::{NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, OneShotHandler, PollParameters};
use libp2p::{Multiaddr, PeerId};
use std::collections::VecDeque;
use std::sync::Arc;
use std::task::{Context, Poll};

mod protocol;

pub use protocol::{BroadcastConfig, Topic};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BroadcastEvent {
    Subscribed(PeerId, Topic),
    Unsubscribed(PeerId, Topic),
    Received(PeerId, Topic, Arc<[u8]>),
}

#[derive(Debug, Default)]
pub struct BroadcastBehaviour {
    config: BroadcastConfig,
    subscriptions: FnvHashSet<Topic>,
    peers: FnvHashMap<PeerId, FnvHashSet<Topic>>,
    topics: FnvHashMap<Topic, FnvHashSet<PeerId>>,
    events: VecDeque<NetworkBehaviourAction<Message, BroadcastEvent>>,
}

impl BroadcastBehaviour {
    pub fn new(config: BroadcastConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    pub fn subscribe(&mut self, topic: Topic) {
        self.subscriptions.insert(topic);
        let msg = Message::Subscribe(topic);
        for peer in self.peers.keys() {
            self.events.push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id: *peer,
                event: msg.clone(),
                handler: NotifyHandler::Any,
            });
        }
    }

    pub fn unsubscribe(&mut self, topic: &Topic) {
        self.subscriptions.remove(topic);
        let msg = Message::Unsubscribe(*topic);
        if let Some(peers) = self.topics.get(topic) {
            for peer in peers {
                self.events.push_back(NetworkBehaviourAction::NotifyHandler {
                    peer_id: *peer,
                    event: msg.clone(),
                    handler: NotifyHandler::Any,
                });
            }
        }
    }

    pub fn broadcast(&mut self, topic: &Topic, msg: Arc<[u8]>) {
        let msg = Message::Broadcast(*topic, msg);
        if let Some(peers) = self.topics.get(topic) {
            for peer in peers {
                self.events.push_back(NetworkBehaviourAction::NotifyHandler {
                    peer_id: *peer,
                    event: msg.clone(),
                    handler: NotifyHandler::Any,
                });
            }
        }
    }
}

impl NetworkBehaviour for BroadcastBehaviour {
    type ProtocolsHandler = OneShotHandler<BroadcastConfig, Message, HandlerEvent>;
    type OutEvent = BroadcastEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _peer: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, peer: &PeerId) {
        self.peers.insert(*peer, FnvHashSet::default());
        for topic in &self.subscriptions {
            self.events.push_back(NetworkBehaviourAction::NotifyHandler {
                peer_id: *peer,
                event: Message::Subscribe(*topic),
                handler: NotifyHandler::Any,
            });
        }
    }

    fn inject_disconnected(&mut self, peer: &PeerId) {
        if let Some(topics) = self.peers.remove(peer) {
            for topic in topics {
                if let Some(peers) = self.topics.get_mut(&topic) {
                    peers.remove(peer);
                }
            }
        }
    }

    fn inject_event(&mut self, peer: PeerId, _: ConnectionId, msg: HandlerEvent) {
        use HandlerEvent::*;
        use Message::*;
        let ev = match msg {
            Rx(Subscribe(topic)) => {
                let peers = self.topics.entry(topic).or_default();
                self.peers.get_mut(&peer).unwrap().insert(topic);
                peers.insert(peer);
                BroadcastEvent::Subscribed(peer, topic)
            }
            Rx(Broadcast(topic, msg)) => BroadcastEvent::Received(peer, topic, msg),
            Rx(Unsubscribe(topic)) => {
                self.peers.get_mut(&peer).unwrap().remove(&topic);
                if let Some(peers) = self.topics.get_mut(&topic) {
                    peers.remove(&peer);
                }
                BroadcastEvent::Unsubscribed(peer, topic)
            }
            Tx => {
                return;
            }
        };
        self.events.push_back(NetworkBehaviourAction::GenerateEvent(ev));
    }

    fn poll(
        &mut self,
        _: &mut Context,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Message, Self::OutEvent>> {
        if let Some(event) = self.events.pop_front() {
            Poll::Ready(event)
        } else {
            Poll::Pending
        }
    }
}

/// Transmission between the `OneShotHandler` and the `BroadcastHandler`.
#[derive(Debug)]
pub enum HandlerEvent {
    /// We received a `Message` from a remote.
    Rx(Message),
    /// We successfully sent a `Message`.
    Tx,
}

impl From<Message> for HandlerEvent {
    fn from(message: Message) -> Self {
        Self::Rx(message)
    }
}

impl From<()> for HandlerEvent {
    fn from(_: ()) -> Self {
        Self::Tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::swarm::AddressRecord;
    use std::sync::{Arc, Mutex};

    struct DummySwarm {
        peer_id: PeerId,
        behaviour: Arc<Mutex<BroadcastBehaviour>>,
        connections: FnvHashMap<PeerId, Arc<Mutex<BroadcastBehaviour>>>,
    }

    impl DummySwarm {
        fn new() -> Self {
            Self {
                peer_id: PeerId::random(),
                behaviour: Default::default(),
                connections: Default::default(),
            }
        }

        fn peer_id(&self) -> &PeerId {
            &self.peer_id
        }

        fn dial(&mut self, other: &mut DummySwarm) {
            self.behaviour.lock().unwrap().inject_connected(other.peer_id());
            self.connections.insert(*other.peer_id(), other.behaviour.clone());
            other.behaviour.lock().unwrap().inject_connected(self.peer_id());
            other.connections.insert(*self.peer_id(), self.behaviour.clone());
        }

        fn next(&self) -> Option<BroadcastEvent> {
            let waker = futures::task::noop_waker();
            let mut ctx = Context::from_waker(&waker);
            let mut me = self.behaviour.lock().unwrap();
            loop {
                match me.poll(&mut ctx, &mut DummyPollParameters) {
                    Poll::Ready(NetworkBehaviourAction::NotifyHandler { peer_id, event, .. }) => {
                        if let Some(other) = self.connections.get(&peer_id) {
                            let mut other = other.lock().unwrap();
                            other.inject_event(*self.peer_id(), ConnectionId::new(0), HandlerEvent::Rx(event));
                        }
                    }
                    Poll::Ready(NetworkBehaviourAction::GenerateEvent(event)) => {
                        return Some(event);
                    }
                    Poll::Ready(_) => panic!(),
                    Poll::Pending => {
                        return None;
                    }
                }
            }
        }

        fn subscribe(&self, topic: Topic) {
            let mut me = self.behaviour.lock().unwrap();
            me.subscribe(topic);
        }

        fn unsubscribe(&self, topic: &Topic) {
            let mut me = self.behaviour.lock().unwrap();
            me.unsubscribe(topic);
        }

        fn broadcast(&self, topic: &Topic, msg: Arc<[u8]>) {
            let mut me = self.behaviour.lock().unwrap();
            me.broadcast(topic, msg);
        }
    }

    struct DummyPollParameters;

    impl PollParameters for DummyPollParameters {
        type SupportedProtocolsIter = std::iter::Empty<Vec<u8>>;
        type ListenedAddressesIter = std::iter::Empty<Multiaddr>;
        type ExternalAddressesIter = std::iter::Empty<AddressRecord>;

        fn supported_protocols(&self) -> Self::SupportedProtocolsIter {
            unimplemented!()
        }

        fn listened_addresses(&self) -> Self::ListenedAddressesIter {
            unimplemented!()
        }

        fn external_addresses(&self) -> Self::ExternalAddressesIter {
            unimplemented!()
        }

        fn local_peer_id(&self) -> &PeerId {
            unimplemented!()
        }
    }

    #[test]
    fn test_broadcast() {
        let topic = Topic::new(b"topic");
        let msg = Arc::new(*b"msg");
        let mut a = DummySwarm::new();
        let mut b = DummySwarm::new();

        a.subscribe(topic);
        a.dial(&mut b);
        assert!(a.next().is_none());
        assert_eq!(b.next().unwrap(), BroadcastEvent::Subscribed(*a.peer_id(), topic));
        b.subscribe(topic);
        assert!(b.next().is_none());
        assert_eq!(a.next().unwrap(), BroadcastEvent::Subscribed(*b.peer_id(), topic));
        b.broadcast(&topic, msg.clone());
        assert!(b.next().is_none());
        assert_eq!(a.next().unwrap(), BroadcastEvent::Received(*b.peer_id(), topic, msg));
        a.unsubscribe(&topic);
        assert!(a.next().is_none());
        assert_eq!(b.next().unwrap(), BroadcastEvent::Unsubscribed(*a.peer_id(), topic));
    }
}
