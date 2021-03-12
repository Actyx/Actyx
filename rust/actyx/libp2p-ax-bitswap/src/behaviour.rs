//! Handles the `/ipfs/bitswap/1.0.0` and `/ipfs/bitswap/1.1.0` protocols. This
//! allows exchanging IPFS blocks.
//!
//! # Usage
//!
//! The `Bitswap` struct implements the `NetworkBehaviour` trait. When used, it
//! will allow providing and receiving IPFS blocks.
#![allow(clippy::type_complexity)]
use super::{
    format::{Message, I, O},
    protocol::BitswapConfig,
};
use crate::block::Block;
use crate::peer_stats::PeerStats;
use crate::util::zip_with_clones;
use cid::Cid;
use futures::{channel::mpsc, prelude::*};
use libp2p::{
    core::connection::ConnectionId,
    swarm::{
        protocols_handler::OneShotHandler, NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, PollParameters,
    },
    Multiaddr, PeerId,
};
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::task::{Context, Poll};
use tokio::time::{Duration, Instant};
use tracing::*;

/// Network behaviour that handles sending and receiving IPFS blocks.
pub struct Bitswap {
    /// Queue of events to report to the user.
    events_sender: mpsc::UnboundedSender<NetworkBehaviourAction<Message<O>, BitswapEvent>>,
    events_receiver: mpsc::UnboundedReceiver<NetworkBehaviourAction<Message<O>, BitswapEvent>>,
    /// Connected peers, with stats
    connected_peers: BTreeMap<PeerId, PeerStats>,
    /// Wanted blocks
    wanted_blocks: BTreeMap<Cid, WantState>,
    /// Regular tick stream to initiate reset of too old WantState::Coming to WantState::Want
    want_reset_ticks: tokio::time::Interval,
}

/// maximum time to allow in state WantState::Coming before retrying with another peer
///
/// Note that retrying with another peer does not mean that we cancel the original request, so if
/// answering takes longer than this it is no big deal. We wil still accept the block.
const MAX_SEND_DURATION: Duration = Duration::from_secs(30);

/// duration after which to resend a have request if nobody answered with an "I have it" answer
///
/// In the worst case we are spamming all of our peers with regular "do you have it" queries,
/// but they are quite cheap, and what else can we do?
const RESEND_DURATION: Duration = Duration::from_secs(120);

/// period in which to run janitorial tasks like resending have requests, cleaning up stale comings, etc.
///
/// The bitswap behaviour will also notify the behaviour user of some tasks with the same period.
const JANITOR_PERIOD: Duration = Duration::from_secs(30);

impl Default for Bitswap {
    fn default() -> Self {
        Self::new()
    }
}

impl Bitswap {
    /// Creates a `Bitswap`.
    pub fn new() -> Self {
        let (events_sender, events_receiver) = mpsc::unbounded();
        Bitswap {
            events_sender,
            events_receiver,
            connected_peers: Default::default(),
            wanted_blocks: Default::default(),
            want_reset_ticks: tokio::time::interval(JANITOR_PERIOD),
        }
    }

    /// Sends blocks to a peer
    pub fn send_blocks(&mut self, peer_id: PeerId, blocks: BTreeSet<Block>) {
        if blocks.is_empty() {
            return;
        }
        debug!("bitswap: send_blocks");
        if self.connected_peers.contains_key(&peer_id) {
            debug!("  queueing block for peer {}", peer_id.to_base58());
            self.send_messages(peer_id, Message::want_response(blocks.into_iter()));
        } else {
            warn!("We tried to send blocks to a peer that we are not connected to. Weird!")
        }
    }

    /// Sends have info to a peer
    pub fn send_haves(&mut self, peer_id: PeerId, info: BTreeMap<Cid, bool>) {
        if info.is_empty() {
            return;
        }
        debug!("bitswap: send_haves");
        if self.connected_peers.contains_key(&peer_id) {
            debug!("  queueing haves for peer {}", peer_id.to_base58());
            self.send_messages(peer_id, Message::have_response(info.into_iter()));
        } else {
            warn!("We tried to send haves to a peer that we are not connected to. Weird!")
        }
    }

    fn cancel_grouped(&mut self, to_cancel: Vec<(PeerId, Cid)>) {
        if to_cancel.is_empty() {
            return;
        }
        debug!("cancel_grouped {}", to_cancel.len());
        #[allow(clippy::mutable_key_type)] // clippy bug #5812
        let mut messages: BTreeMap<PeerId, Message<O>> = BTreeMap::new();
        // queue cancel requests to everything that was in coming state.
        for (peer_id, cid) in to_cancel {
            debug!("queuing cancel for {}", peer_id.to_base58());
            let message = messages.entry(peer_id).or_default();
            message.add_cancel_block(&cid);
        }
        // send out the grouped cancel requests
        for (peer_id, message) in messages {
            self.send_message(peer_id, message)
        }
    }

    pub fn want_cleanup(&mut self, keep: impl Fn(&Cid) -> bool) {
        // find set of cids to remove, because they are no longer needed from "the outside"
        let to_remove: Vec<Cid> = self.wanted_blocks.keys().filter(|cid| !keep(cid)).cloned().collect();

        // find the set of cids that we actually have to cancel, because they were in coming state. We don't cancel
        // want state because the cancellation is not worth it.
        //
        // as a side effect, we also remove the entries from wanted_blocks that we don't need anymore
        let to_cancel: Vec<(PeerId, Cid)> = to_remove
            .into_iter()
            .filter_map(|cid| {
                self.wanted_blocks.remove(&cid).and_then(|state| match state {
                    WantState::Want { .. } => None,
                    WantState::Coming { from, .. } => Some((from, cid)),
                })
            })
            .collect();
        self.cancel_grouped(to_cancel);
    }

    /// User wants us to get a bunch of blocks in whatever way
    pub fn want_blocks(&mut self, cids: BTreeSet<Cid>) {
        // get fresh want blocks, and also update the want state for them
        let now = Instant::now();
        let wanted_cids = cids.into_iter().filter_map(|cid| {
            match self.wanted_blocks.entry(cid) {
                Entry::Vacant(e) => {
                    // create a new entry
                    let cid = *e.key();
                    e.insert(WantState::want(now));
                    Some(cid)
                }
                _ => None,
            }
        });
        let messages = Message::have_query(wanted_cids);
        // if all things on our want list are already presumably coming from somebody, there is nothing to do
        if messages.is_empty() {
            return;
        }
        // ask all our current peers if they have the blocks
        let peer_ids = self.connected_peers.keys().cloned().collect();
        for (peer_id, messages) in zip_with_clones(peer_ids, messages) {
            self.send_messages(peer_id, messages);
        }
    }

    /// Sends the wantlist to a number of peers.
    ///
    /// We just ask the peer about the blocks and do not ask him to send the blocks.
    fn send_want_list(&mut self, peer_ids: Vec<PeerId>) {
        let wanted_cids = self.wanted_blocks.iter().filter_map(|(cid, state)| match state {
            WantState::Want { .. } => Some(*cid),
            _ => None,
        });
        // create a message
        let messages = Message::have_query(wanted_cids);
        // if all things on our want list are already presumably coming from somebody, there is nothing to do
        if messages.is_empty() {
            return;
        }
        for (peer_id, messages) in zip_with_clones(peer_ids, messages) {
            self.send_messages(peer_id, messages);
        }
    }

    /// Removes the cids from the want list and sends the appropriate cancel requests
    ///
    /// We are not sending cancel requests to the sender.
    fn cancel_blocks<'a>(&mut self, sender: &PeerId, cids: impl IntoIterator<Item = &'a Cid>) {
        // remove the block from our wanted_blocks list so we don't ask for it again
        let to_cancel: Vec<(PeerId, Cid)> = cids
            .into_iter()
            .filter_map(|cid| {
                self.wanted_blocks.remove(cid).and_then(|state| match state {
                    WantState::Coming { from, .. } if from != *sender => Some((from, *cid)),
                    _ => None,
                })
            })
            .collect();
        self.cancel_grouped(to_cancel);
    }

    /// new peer is connected. We send the entire want list except for things
    /// that are currently confirmed to be handled by other peers.
    fn peer_connected(&mut self, peer_id: &PeerId) {
        let ledger = PeerStats::new();
        if self.connected_peers.insert(*peer_id, ledger).is_none() {
            // only send want list if this was the 1st connection to the peer
            self.send_want_list(vec![*peer_id]);
        }
    }

    /// look at all cids that are in an in flight state (WantState::Coming). If they are too old,
    /// we reset them to Wantstate::Want and ask all peers again.
    fn remove_stale_inflight(&mut self, now: Instant) {
        // tell the users of this behaviour that now would be a good time to clean up the want list
        self.send_event(BitswapEvent::WantCleanup);

        // figure out for which cids on the want list we need to request again
        let to_request = self
            .wanted_blocks
            .iter_mut()
            .filter_map(|(cid, state)| match &state {
                WantState::Coming { from, since } => {
                    let delay: Duration = now - *since;
                    if delay >= MAX_SEND_DURATION {
                        let res = Some((*cid, Some(*from), delay));
                        *state = WantState::Want { since: now };
                        res
                    } else {
                        None
                    }
                }
                WantState::Want { since } => {
                    let delay: Duration = now - *since;
                    if delay >= RESEND_DURATION {
                        let res = Some((*cid, None, delay));
                        *state = WantState::Want { since: now };
                        res
                    } else {
                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        // nothing to do, return so we don't send empty msgs
        if to_request.is_empty() {
            return;
        }

        // only log once we are sure that there is actually something to do
        info!("reset {} want states due to inactivity", to_request.len());

        // update the stats for the promise breakers
        for (_, from, delay) in to_request.iter() {
            if let Some(from) = from {
                if let Some(stats) = self.connected_peers.get_mut(from) {
                    stats.add_missed_coming(*delay)
                }
            }
        }

        let to_request = to_request.into_iter().map(|(cid, _, _)| cid);

        // request from all blocks again. We also give the promise breaker another chance
        let messages = Message::have_query(to_request);
        let peer_ids: Vec<PeerId> = self.connected_peers.keys().cloned().collect();
        for (peer_id, messages) in zip_with_clones(peer_ids, messages) {
            self.send_messages(peer_id, messages);
        }
    }

    /// called when a peer is disconnected.
    ///
    /// this shuffles everything we were expecting this peer to do to other peers.
    fn peer_disconnected(&mut self, peer_id: &PeerId) {
        let now = Instant::now();
        self.connected_peers.remove(peer_id);
        // find all cids which were coming from the just disconnected peer
        let to_request = self.wanted_blocks.iter_mut().filter_map(|(cid, state)| {
            match state {
                WantState::Coming { from, .. } if from == peer_id => {
                    // reset the state to want state want, we are not going to get it from this peer
                    *state = WantState::Want { since: now };
                    Some(*cid)
                }
                _ => None,
            }
        });
        // send a have_block request for all blocks to all remaining peers
        //
        // we have to do this since the peers might have already answered while we were in Coming state for that cid
        let messages = Message::have_query(to_request);
        if messages.is_empty() {
            return;
        }
        let peer_ids: Vec<PeerId> = self.connected_peers.keys().cloned().collect();
        for (peer_id, messages) in zip_with_clones(peer_ids, messages) {
            self.send_messages(peer_id, messages);
        }
    }

    /// Send messages to a peer
    fn send_messages(&mut self, peer_id: PeerId, messages: Vec<Message<O>>) {
        debug!("send_messages {} {}", peer_id, messages.len());
        for message in messages {
            self.send_message(peer_id, message);
        }
    }

    /// Send a message to a peer
    fn send_message(&mut self, peer_id: PeerId, message: Message<O>) {
        // unbounded_send can not fail unless the receiver is dropped.
        // this is not possible here because the receiver is in the same struct.
        let _ = self
            .events_sender
            .unbounded_send(NetworkBehaviourAction::NotifyHandler {
                peer_id,
                event: message,
                handler: NotifyHandler::Any,
            });
    }

    /// Send an event to the users of this behaviour
    fn send_event(&mut self, event: BitswapEvent) {
        // unbounded_send can not fail unless the receiver is dropped.
        // this is not possible here because the receiver is in the same struct.
        let _ = self
            .events_sender
            .unbounded_send(NetworkBehaviourAction::GenerateEvent(event));
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum BitswapEvent {
    /// we have gotten some blocks
    BlocksReceived(BTreeSet<Block>),
    /// we have gotten a request for sending blocks
    BlockWanted { peer: PeerId, cids: BTreeSet<Cid> },
    /// we have gotten a request for checking if we have a set of blocks
    BlockHave { peer: PeerId, cids: BTreeSet<Cid> },
    /// we want the outside to tell us what wants are still valid
    WantCleanup,
}

impl NetworkBehaviour for Bitswap {
    type ProtocolsHandler = OneShotHandler<BitswapConfig, Message<O>, InnerMessage>;
    type OutEvent = BitswapEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, peer_id: &PeerId) {
        self.peer_connected(peer_id);
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId) {
        self.peer_disconnected(peer_id);
    }

    fn inject_event(&mut self, source: PeerId, _connection_id: ConnectionId, event: InnerMessage) {
        let message = match event {
            InnerMessage::Rx(message) => message,
            InnerMessage::Tx => {
                return;
            }
        };

        // Update the stats
        if let Some(ledger) = self.connected_peers.get_mut(&source) {
            ledger.update_incoming_stats(&message);
        }

        // we got some blocks
        if !message.blocks.is_empty() {
            // cancel the want for all blocks
            self.cancel_blocks(&source, message.blocks.iter().map(|b| b.cid()));
            // push back a single msg containing all blocks for storage
            //
            // should we store stuff we have not asked for?
            self.send_event(BitswapEvent::BlocksReceived(message.blocks.into_iter().collect()));
        }

        // we got some block presence answers
        if !message.block_presences.is_empty() {
            // figure out which blocks we immediately want to ask for, and also as a side effect update the want state
            let to_request = message
                .block_presences
                .iter()
                .filter_map(|(cid, have)| if *have { Some(cid) } else { None })
                .filter_map(|cid| {
                    if let Some(state @ WantState::Want { .. }) = self.wanted_blocks.get_mut(cid) {
                        // change the want state to coming, marking with the current time
                        *state = WantState::Coming {
                            from: source,
                            since: Instant::now(),
                        };
                        // we want this
                        Some(*cid)
                    } else {
                        // we either did not want this in the first place, or we are already getting it from someone else
                        None
                    }
                });
            // there are some things we want from the sender which we are not yet getting from anybody else
            let messages = Message::want_query(to_request);
            self.send_messages(source, messages);
        }

        // we got some want requests, dispatch them so the store can answer
        if !message.want.is_empty() {
            self.send_event(BitswapEvent::BlockWanted {
                peer: source,
                cids: message.want,
            });
        }

        // we got some have requests, dispatch them so the store can answer
        if !message.have.is_empty() {
            self.send_event(BitswapEvent::BlockHave {
                peer: source,
                cids: message.have,
            });
        }
    }

    fn poll(
        &mut self,
        ctx: &mut Context,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Message<O>, BitswapEvent>> {
        if let Poll::Ready(instant) = self.want_reset_ticks.poll_tick(ctx) {
            self.remove_stale_inflight(instant);
        }
        if let Poll::Ready(Some(event)) = self.events_receiver.poll_next_unpin(ctx) {
            if let NetworkBehaviourAction::NotifyHandler {
                peer_id,
                handler,
                event,
            } = event
            {
                match self.connected_peers.get_mut(&peer_id) {
                    None => Poll::Pending,
                    Some(ref mut stats) => {
                        stats.update_outgoing_stats(&event);
                        Poll::Ready(NetworkBehaviourAction::NotifyHandler {
                            peer_id,
                            handler,
                            event,
                        })
                    }
                }
            } else {
                Poll::Ready(event)
            }
        } else {
            Poll::Pending
        }
    }
}

/// Transmission between the `OneShotHandler` and the `BitswapHandler`.
#[derive(Debug)]
pub enum InnerMessage {
    /// We received a `Message` from a remote.
    Rx(Message<I>),
    /// We successfully sent a `Message`.
    Tx,
}

impl From<Message<I>> for InnerMessage {
    #[inline]
    fn from(message: Message<I>) -> InnerMessage {
        InnerMessage::Rx(message)
    }
}

impl From<()> for InnerMessage {
    #[inline]
    fn from(_: ()) -> InnerMessage {
        InnerMessage::Tx
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum WantState {
    /// We have asked our peers if they have this
    Want {
        /// The instant when sent the have request
        ///
        /// This is useful to check if all our peers have forgotten the question, to ask again
        since: Instant,
    },
    /// We have gotten a confirmation from a peer and have asked this peer to send us the actual block
    Coming {
        /// the peer from which we have requested the data
        from: PeerId,
        /// The instant when we went the want request.
        ///
        /// This is useful to check if the client does not send the data after a reasonable time
        since: Instant,
    },
}

impl WantState {
    fn want(since: Instant) -> Self {
        Self::Want { since }
    }

    #[cfg(test)]
    fn is_want(&self) -> bool {
        matches!(self, WantState::Want { .. })
    }

    #[cfg(test)]
    fn is_coming(&self) -> bool {
        matches!(self, WantState::Coming { .. })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::mutable_key_type)] // clippy bug #5812
    #![allow(clippy::redundant_clone)]
    use super::*;
    use crate::codecs::DAG_PROTOBUF;
    use futures_test::task::noop_context;
    use libp2p::{
        swarm::{AddressRecord, NetworkBehaviour},
        Multiaddr,
    };
    use maplit::*;
    use multihash::MultihashDigest;
    use rand::Rng;
    use std::collections::HashSet;

    // call poll on a bitswap with dummy params
    fn poll(bitswap: &mut Bitswap, ctx: &mut Context) -> Poll<NetworkBehaviourAction<Message<O>, BitswapEvent>> {
        let mut params = DummyPollParameters;
        bitswap.poll(ctx, &mut params)
    }

    /// call to drain the behaviour until it returns pending.
    ///
    /// collect events and messages in a hashset, because relying on the exact order in the tests would be fragile
    fn poll_until_pending(bitswap: &mut Bitswap) -> (HashSet<BitswapEvent>, HashSet<(PeerId, Message<O>)>) {
        let mut ctx = noop_context();
        let mut events: HashSet<BitswapEvent> = Default::default();
        let mut messages: HashSet<(PeerId, Message<O>)> = Default::default();
        while let Poll::Ready(ev) = poll(bitswap, &mut ctx) {
            match ev {
                NetworkBehaviourAction::GenerateEvent(e) => {
                    events.insert(e);
                }
                NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    event,
                    handler: NotifyHandler::Any,
                } => {
                    messages.insert((peer_id, event));
                }
                ev => {
                    panic!("Unexpected NetworkBehaviourAction from Bitswap: {:?}", ev);
                }
            };
        }
        (events, messages)
    }

    fn inject(bitswap: &mut Bitswap, peer_id: PeerId, value: Message<O>) {
        bitswap.inject_event(peer_id, ConnectionId::new(0), InnerMessage::Rx(value.into_input()))
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

    macro_rules! assert_pending {
        ($bitswap:ident) => {{
            let (e, m) = poll_until_pending(&mut $bitswap);
            assert_eq!(e, hashset! {});
            assert_eq!(m, hashset! {});
        }};
    }

    fn peer() -> PeerId {
        PeerId::random()
    }

    fn cid() -> Cid {
        let mut rng = rand::thread_rng();
        let mut data = [0u8; 32];
        rng.fill(&mut data);
        Cid::new(cid::Version::V1, DAG_PROTOBUF, multihash::Code::Sha2_256.digest(&data)).unwrap()
    }

    /// fn to make a have query without all the annoying cloning
    fn have_query(cids: &[&Cid]) -> Message<O> {
        Message::have_query(cids.iter().cloned().cloned()).pop().unwrap()
    }

    /// fn to make a want query without all the annoying cloning
    fn want_query(cids: &[&Cid]) -> Message<O> {
        Message::want_query(cids.iter().cloned().cloned()).pop().unwrap()
    }

    /// fn to make a cancel command without all the annoying cloning
    fn cancel_command(cids: &[&Cid]) -> Message<O> {
        Message::cancel_command(cids.iter().cloned().cloned()).pop().unwrap()
    }

    /// fn to make a have response with value true without all the annoying cloning
    fn have_response(cids: &[&Cid]) -> Message<O> {
        Message::have_response(cids.iter().cloned().cloned().map(|c| (c, true)))
            .pop()
            .unwrap()
    }

    /// fn to make a want response with dummy blocks without all the annoying cloning
    fn want_response(cids: &[&Cid]) -> Message<O> {
        Message::want_response(cids.iter().cloned().cloned().map(|c| Block::new(vec![], c)))
            .pop()
            .unwrap()
    }

    #[tokio::test]
    async fn connect_disconnect() {
        let mut bitswap = Bitswap::new();
        let p1 = peer();
        // connect
        bitswap.peer_connected(&p1);
        // connect with empty want list should not trigger any activity
        assert_pending!(bitswap);
        // check that the peer is marked as connected
        assert!(bitswap.connected_peers.contains_key(&p1));
        // disconnect
        bitswap.peer_disconnected(&p1);
        // disconnect with empty want list should not trigger any activity
        assert_pending!(bitswap);
        // check that the peer is gone from connected_peers
        assert!(!bitswap.connected_peers.contains_key(&p1));
    }

    #[tokio::test]
    async fn want_connect_disconnect() {
        let mut bitswap = Bitswap::new();

        let p1 = peer();
        let c1 = cid();
        bitswap.want_blocks(btreeset! { c1 });
        // wanting with no peers should not trigger any activity
        assert_pending!(bitswap);
        // connect
        bitswap.peer_connected(&p1);
        let (e, m) = poll_until_pending(&mut bitswap);
        // connect with a want list should send the entire want list as have query
        assert_eq!(m, hashset! { (p1, have_query(&[&c1])) });
        assert_eq!(e, hashset! {});

        // connect again to the same peer
        bitswap.peer_connected(&p1);
        // should be a noop
        assert_pending!(bitswap);

        // check that the peer is marked as connected
        assert!(bitswap.connected_peers.contains_key(&p1));
        // // disconnect
        bitswap.peer_disconnected(&p1);
        // connect with no remaining peers
        assert_pending!(bitswap);
        // check that the peer is gone from connected_peers
        assert!(!bitswap.connected_peers.contains_key(&p1));
    }

    #[tokio::test]
    async fn scenario_have_want_simple() {
        let mut bitswap = Bitswap::new();

        let p1 = peer();
        let p2 = peer();
        let c1 = cid();
        let c2 = cid();

        bitswap.want_blocks(btreeset! { c1, c2 });
        assert_pending!(bitswap);

        // connect p1
        bitswap.peer_connected(&p1);
        assert!(bitswap.connected_peers.contains_key(&p1));
        let (e, m) = poll_until_pending(&mut bitswap);
        // connect with an active want list should send the entire want list as have query
        assert_eq!(m, hashset! { (p1, have_query(&[&c1, &c2])) },);
        assert_eq!(e, hashset! {});

        // connect p2
        bitswap.peer_connected(&p2);
        assert!(bitswap.connected_peers.contains_key(&p2));
        let (e, m) = poll_until_pending(&mut bitswap);
        // connect with an active want list should send the entire want list as have query
        assert_eq!(m, hashset! { (p2, have_query(&[&c1, &c2])) },);
        assert_eq!(e, hashset! {});

        // p1 confirms that it has c1
        inject(&mut bitswap, p1, have_response(&[&c1]));
        let (e, m) = poll_until_pending(&mut bitswap);
        // a have response for something we want should be immediately followed by a want query
        assert_eq!(m, hashset! { (p1, want_query(&[&c1])) },);
        assert_eq!(e, hashset! {});

        // p1 actually sends c1
        let msg = want_response(&[&c1]);
        inject(&mut bitswap, p1, msg.clone());
        let (e, m) = poll_until_pending(&mut bitswap);
        assert_eq!(e, hashset! { BitswapEvent::BlocksReceived(msg.blocks().clone()) });
        assert_eq!(m, hashset! {});
        // want for just delivered block c1 should be gone
        assert!(!bitswap.wanted_blocks.contains_key(&c1),);
        // want for block c2 should still be there
        assert!(bitswap.wanted_blocks.contains_key(&c2),);
    }

    /// tests a race scenario. We are asking for a cid, but two peers have it.
    /// Only the first to answer gets the request to send it. But for whatever reason we get it from the second peer.
    /// This causes the want request to the first peer to be canceled, and we get the block. The end.
    #[tokio::test]
    async fn scenario_have_want_race() {
        // manually control time
        tokio::time::pause();
        let mut bitswap = Bitswap::new();
        let p1 = peer();
        let p2 = peer();
        let c1 = cid();
        let c2 = cid();

        // want c1 and c2, then connect p1 and p2
        bitswap.want_blocks(btreeset! { c1, c2 });
        bitswap.peer_connected(&p1);
        bitswap.peer_connected(&p2);
        poll_until_pending(&mut bitswap);

        // p1 and p2 have c1, in that order
        inject(&mut bitswap, p1, have_response(&[&c1]));
        inject(&mut bitswap, p1, have_response(&[&c1]));
        poll_until_pending(&mut bitswap);

        // want state should be coming from p1. The reason the equality for since works is because we manually control time!
        assert_eq!(
            bitswap.wanted_blocks[&c1],
            WantState::Coming {
                from: p1,
                since: Instant::now()
            }
        );

        // p2(!) has the data
        let msg = want_response(&[&c1]);
        inject(&mut bitswap, p2, msg.clone());
        let (e, m) = poll_until_pending(&mut bitswap);
        assert_eq!(m, hashset! { (p1, cancel_command(&[&c1])) });
        assert_eq!(e, hashset! { BitswapEvent::BlocksReceived(msg.blocks().clone()) });
        assert!(!bitswap.wanted_blocks.contains_key(&c1),);
        assert!(bitswap.wanted_blocks.contains_key(&c2),);
    }

    /// tests a timeout scenario.
    ///
    /// we have gotten a promise from p1 that it has c1. But we don't actually get the content for c1 for some time.
    /// we want to switch the want state back to a generic want, and resent a have query to all peers.
    #[tokio::test]
    async fn scenario_want_timeout() {
        // manually control time
        tokio::time::pause();
        let mut bitswap = Bitswap::new();
        let p1 = peer();
        let p2 = peer();
        let c1 = cid();
        let c2 = cid();

        // want c1 and c2, then connect p1 and p2
        bitswap.want_blocks(btreeset! { c1, c2 });
        bitswap.peer_connected(&p1);
        bitswap.peer_connected(&p2);
        poll_until_pending(&mut bitswap);

        // p1 and p2 have c1, in that order
        // at this point the clock is running, and we expect to hear from p1.
        inject(&mut bitswap, p1, have_response(&[&c1]));
        inject(&mut bitswap, p1, have_response(&[&c1]));
        assert!(bitswap.wanted_blocks[&c1].is_coming());
        poll_until_pending(&mut bitswap);

        // advance the time so the janitor runs, and run the janitor by calling poll
        tokio::time::advance(JANITOR_PERIOD + Duration::from_millis(1)).await;
        let (e, m) = poll_until_pending(&mut bitswap);

        // check that the want state has reverted to want
        assert!(bitswap.wanted_blocks[&c1].is_want());

        // check that due to the janior running once, we got a WantCleanup event
        assert_eq!(e, hashset! { BitswapEvent::WantCleanup });
        // check that the janitor has sent have queries to the two connected peers
        assert_eq!(
            m,
            hashset! {
                (p1, have_query(&[&c1])),
                (p2, have_query(&[&c1])),
            }
        );
    }

    /// tests a timeout scenario.
    ///
    /// we send have requests to all peers. They don't answer for a long time. We send the have requests again. The end.
    #[tokio::test]
    async fn scenario_want_resend() {
        // manually control time
        tokio::time::pause();
        let mut bitswap = Bitswap::new();
        let p1 = peer();
        let p2 = peer();
        let c1 = cid();
        let c2 = cid();

        // want c1 and c2, then connect p1 and p2
        bitswap.want_blocks(btreeset! { c1, c2 });
        bitswap.peer_connected(&p1);
        bitswap.peer_connected(&p2);
        let (_, m) = poll_until_pending(&mut bitswap);
        // check that both peers have gotten have queries for the complete want list
        assert_eq!(
            m,
            hashset! {
                (p1, have_query(&[&c1, &c2])),
                (p2, have_query(&[&c1, &c2])),
            }
        );

        // advance the time
        tokio::time::advance(RESEND_DURATION + Duration::from_millis(1)).await;
        let (e, m) = poll_until_pending(&mut bitswap);

        // check that due to the janior running once, we got a WantCleanup event
        assert_eq!(e, hashset! { BitswapEvent::WantCleanup });
        // check that the janitor has sent have queries to the two connected peers
        assert_eq!(
            m,
            hashset! {
                (p1, have_query(&[&c1, &c2])),
                (p2, have_query(&[&c1, &c2])),
            }
        );
    }

    /// We are getting some data from a node. Then it disconnects. We need to ask somebody else. The end.
    #[tokio::test]
    async fn scenario_disconnect_resend() {
        // manually control time
        tokio::time::pause();
        let mut bitswap = Bitswap::new();
        let p1 = peer();
        let p2 = peer();
        let c1 = cid();
        let c2 = cid();

        // want c1 and c2, then connect p1 and p2
        bitswap.want_blocks(btreeset! { c1, c2 });
        bitswap.peer_connected(&p1);
        bitswap.peer_connected(&p2);
        poll_until_pending(&mut bitswap);

        // p1 says that he has c1
        inject(&mut bitswap, p1, have_response(&[&c1]));
        assert!(bitswap.wanted_blocks[&c1].is_coming());
        poll_until_pending(&mut bitswap);

        // p1 disconnects!
        bitswap.peer_disconnected(&p1);

        // check that we are asking p2 for c1, since we are not going to get it from p1 anymore
        let (e, m) = poll_until_pending(&mut bitswap);
        assert_eq!(e, hashset! {});
        assert_eq!(m, hashset! { (p2, have_query(&[&c1])) });
    }

    /// We are getting some data from a node. The future that caused us to want the data is dropped.
    /// We have to clean up our want list and cancel the request.
    #[tokio::test]
    async fn scenario_canceled_receiver() {
        let mut bitswap = Bitswap::new();
        let p1 = peer();
        let p2 = peer();
        let c1 = cid();
        let c2 = cid();
        let c3 = cid();

        // want c1 and c2 and c3, then connect p1 and p2
        bitswap.want_blocks(btreeset! { c1, c2, c3 });
        bitswap.peer_connected(&p1);
        bitswap.peer_connected(&p2);
        poll_until_pending(&mut bitswap);

        // p2 says that he has c1
        inject(&mut bitswap, p2, have_response(&[&c1]));
        assert!(bitswap.wanted_blocks[&c1].is_coming());
        poll_until_pending(&mut bitswap);

        // behaviour user calls want cleanup. c1 and c2 are expired.
        // usually this is triggered by the BitswapEvent::WantCleanup event, but we just trigger it directly
        bitswap.want_cleanup(|cid| *cid == c3);

        // check that the wants for c1 and c2 are gone
        assert!(!bitswap.wanted_blocks.contains_key(&c1));
        assert!(!bitswap.wanted_blocks.contains_key(&c2));
        assert!(bitswap.wanted_blocks.contains_key(&c3));

        // check that we sent out a cancel command to p2 since we don't need c1 anymore
        let (e, m) = poll_until_pending(&mut bitswap);
        assert_eq!(e, hashset! {});
        assert_eq!(m, hashset! { (p2, cancel_command(&[&c1])) });
    }
}
