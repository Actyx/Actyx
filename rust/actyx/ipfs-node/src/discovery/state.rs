//! a state machine that takes information about the node itself and information about
//! other nodes to build a view of available addresses of a swarm.
//!
//! this object is completely passive and synchronous. It must be updated or queried by calling the
//! appropriate methods.
#![allow(clippy::redundant_clone)]
use super::PRUNE_ADDRESS_AFTER;
use crate::{
    discovery::formats::{MultiaddrIo, PeerIdIo},
    discovery::protocol::{DiscoveryMessage, NodeInfo, NodeStats},
    discovery::util::strip_peer_id,
};
use libipld::Multihash;
use libp2p::{
    core::ConnectedPoint,
    multiaddr::Protocol,
    swarm::{PollParameters, SwarmEvent},
    Multiaddr, PeerId,
};
use maplit::btreemap;
use serde::Serialize;
use std::{
    cmp,
    collections::{btree_map::Entry, BTreeMap, BTreeSet},
    fmt,
};
use tokio::time::Instant;
use tracing::*;

#[derive(Debug, Clone, Serialize)]
#[serde(into = "SwarmStateIo")]
pub struct SwarmState {
    /// Own peer id
    peer_id: PeerId,
    /// Own addresses
    pub(crate) listen_addrs: BTreeSet<Multiaddr>,
    /// Externally observed addresses
    pub(crate) observed_addrs: BTreeSet<Multiaddr>,
    /// Manually configured addresses to announce
    ///
    /// if this is set, these will be the only addresses that will be announced!
    pub(crate) announce_addrs: BTreeSet<Multiaddr>,
    /// Information about other peers
    peers: BTreeMap<PeerId, PeerState>,
}

impl SwarmState {
    pub fn new(peer_id: PeerId) -> Self {
        Self {
            peer_id,
            listen_addrs: Default::default(),
            observed_addrs: Default::default(),
            announce_addrs: Default::default(),
            peers: Default::default(),
        }
    }

    #[cfg(test)]
    pub fn listen_addrs(&self) -> &BTreeSet<Multiaddr> {
        &self.listen_addrs
    }

    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    pub fn add_bootstrap(&mut self, mut addr: Multiaddr) {
        if let Some(peer_id) = strip_peer_id(&mut addr) {
            self.add_listen_addr(peer_id, addr, AddressProvenance::Bootstrap);
        } else {
            error!("bootstrap address without peer id not supported");
        }
    }

    pub fn add_announce(&mut self, mut addr: Multiaddr) {
        if strip_peer_id(&mut addr).is_none() {
            self.announce_addrs.insert(addr);
        } else {
            error!("announce address with peer id not supported");
        }
    }

    /// adds a swarm event to the state. Returns true if the event resulted in a state change.
    pub fn add_swarm_event<T, E>(&mut self, ev: &SwarmEvent<T, E>) -> bool {
        debug!("add_swarm_event {:?}", DebugSwarmEvent(ev));
        match ev {
            SwarmEvent::NewListenAddr(address) => self.listen_addrs.insert(address.clone()),
            SwarmEvent::ExpiredListenAddr(address) => self.listen_addrs.remove(address),
            SwarmEvent::Dialing(peer_id) => self.set_connection_state(*peer_id, ConnectionState::Connecting),
            SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established: _,
            } => {
                // num_established is guaranteed to be > 0
                if let Some(addr) = dialed_addr(endpoint) {
                    self.set_address_state(*peer_id, addr.clone(), AddressState::connected());
                }
                self.set_connection_state(*peer_id, ConnectionState::Connected)
            }
            SwarmEvent::ConnectionClosed {
                peer_id,
                num_established,
                endpoint,
                ..
            } => {
                // num_established is guaranteed to be > 0
                let state = if *num_established == 0 {
                    ConnectionState::Disconnected
                } else {
                    ConnectionState::Connected
                };
                if let Some(addr) = dialed_addr(endpoint) {
                    self.set_address_state(*peer_id, addr.clone(), AddressState::disconnected());
                }
                self.set_connection_state(*peer_id, state)
            }
            SwarmEvent::UnreachableAddr {
                peer_id,
                address,
                attempts_remaining,
                ..
            } => {
                self.set_address_state(*peer_id, address.clone(), AddressState::disconnected());
                if *attempts_remaining == 0 {
                    self.set_connection_state(*peer_id, ConnectionState::Disconnected)
                } else {
                    false
                }
            }
            SwarmEvent::IncomingConnection {
                local_addr: _,
                send_back_addr: _,
            } => false,
            SwarmEvent::IncomingConnectionError {
                local_addr: _,
                send_back_addr: _,
                ..
            } => false,
            SwarmEvent::UnknownPeerUnreachableAddr { address: _, error: _ } => false,
            SwarmEvent::BannedPeer {
                peer_id: _,
                endpoint: _,
            } => false,
            SwarmEvent::ListenerClosed { addresses, reason } => {
                warn!("Listener closed {:?} {:?}", addresses, reason);
                false
            }
            SwarmEvent::ListenerError { error: _ } => false,
            SwarmEvent::Behaviour(_) => false,
        }
    }

    /// add a discovery message to the state.
    ///
    /// returns a set of multiaddrs that we have learned about that might be worth dialing
    pub fn add_discovery_message(&mut self, msg: DiscoveryMessage) -> Vec<Multiaddr> {
        match msg {
            DiscoveryMessage::NodeInfo(info) => self.include_node_info(info),
            DiscoveryMessage::NewListenAddr(info) => {
                self.add_listen_addr(info.peer, info.addr, AddressProvenance::Discovery)
            }
            DiscoveryMessage::ExpiredListenAddr(info) => self.remove_listen_addr(info.peer, &info.addr),
        }
    }

    /// GC expired addresses, keeping the bootstrap'ed ones. Also expire peers.
    /// An address expires if it has been disconnected for longer than GC_INTERVAL
    /// A peer expires if it has no addresses left.
    pub fn gc_expired_addresses_and_peers(&mut self) {
        let now = Instant::now();
        for peer in self.peers.iter_mut() {
            let addresses_to_remove: Vec<Multiaddr> = peer
                .1
                .addresses
                .iter()
                .filter_map(|(k, v)| if v.has_lapsed(now) { Some(k.clone()) } else { None })
                .collect();
            for k in addresses_to_remove {
                peer.1.addresses.remove(&k);
            }
        }
        let to_remove: Vec<PeerId> = self
            .peers
            .iter()
            .filter_map(|(k, v)| if v.addresses.is_empty() { Some(*k) } else { None })
            .collect();
        // println!("Will remove an expired peer: {:?}", &to_remove);
        for k in to_remove {
            self.peers.remove(&k);
        }
    }

    /// Attempt to prune an expired MDNS address
    pub fn check_and_prune_expired_mdns_address(&mut self, peer_id: &PeerId, maddr: &Multiaddr) {
        if let Some(v) = self.peers.get_mut(peer_id) {
            if let Some(ps) = v.addresses.get_mut(maddr) {
                if ps.provenance == AddressProvenance::MDNS {
                    debug!("removing expired address from MDns: peer:{} address:{}", peer_id, maddr);
                    v.addresses.remove(maddr);
                }
            }
        }
    }

    /// List of peers that we know about yet are not connected to according to this state
    pub fn disconnected_peers(&self) -> Vec<PeerId> {
        self.peers
            .iter()
            .filter_map(|(k, v)| {
                if v.connection_state == ConnectionState::Disconnected {
                    Some(*k)
                } else {
                    None
                }
            })
            .collect()
    }

    /// List of peers that we know about that are connected according to this state
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.peers
            .iter()
            .filter_map(|(k, v)| {
                if v.connection_state == ConnectionState::Connected {
                    Some(*k)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Everything we know about ourselves
    pub fn own_node_info(&self) -> NodeInfo {
        #![allow(clippy::mutable_key_type)] // clippy bug #5812
        let own_addrs = if self.announce_addrs.is_empty() {
            // TODO: sort in some useful way. Most promising addrs should come first
            self.observed_addrs
                .iter()
                .chain(self.listen_addrs.iter())
                .cloned()
                .collect::<BTreeSet<_>>()
        } else {
            self.announce_addrs.clone()
        };
        NodeInfo {
            addresses: btreemap! {
                self.peer_id => own_addrs,
            },
            stats: NodeStats {
                connected_peers: self.connected_peers().len() as u64,
                known_peers: self.peers.len() as u64,
            },
        }
    }

    /// Returns all addresses that we know for this peer.
    ///
    /// The returned addresses are guaranteed not to contain the peer id, so they can be dialed
    /// by rust-libp2p. If you want to dial them with go-ipfs, you have to add them again.
    pub fn addresses_of_peer(&self, peer_id: &PeerId) -> BTreeSet<Multiaddr> {
        self.peers
            .get(peer_id)
            .map(|peer_state| peer_state.addresses.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Returns all addresses that we know for this peer.
    ///
    /// The returned addresses include the peer id, so they can be dialed by go-ipfs.
    pub fn addresses_of_peer_with_peerid(&self, peer_id: &PeerId) -> BTreeSet<Multiaddr> {
        let res = self.addresses_of_peer(peer_id);
        let mh = Multihash::from_bytes(&peer_id.to_bytes()).expect("valid peer_id");
        res.into_iter()
            .map(|mut addr| {
                addr.push(Protocol::P2p(mh));
                addr
            })
            .collect()
    }

    /// Returns all addresses that we know for this peer together with their provenances.
    ///
    /// The returned addresses are guaranteed not to contain the peer id, so they can be dialed
    /// by rust-libp2p. If you want to dial them with go-ipfs, you have to add them again.
    #[cfg(test)] // used in tests atm
    pub fn addresses_of_peer_with_provenance(&self, peer_id: &PeerId) -> Vec<(Multiaddr, AddressProvenance)> {
        self.peers
            .get(peer_id)
            .map(|peer_state| {
                peer_state
                    .addresses
                    .iter()
                    .map(|(k, v)| (k.clone(), v.provenance))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// completely set the connected addrs from an external source
    ///
    /// all addresses not included here will be assumed to be disconnected
    pub fn set_connected_addrs(&mut self, addrs: impl IntoIterator<Item = Multiaddr>) {
        #[allow(clippy::mutable_key_type)] // clippy bug #5812
        // this is done in a bit of a convoluted way to ensure that the state of each address is set exactly once.
        // if we would set the state to disconnected and then back to connected, we would lose the time info.
        let mut connected: BTreeMap<PeerId, BTreeSet<Multiaddr>> = Default::default();
        for (peer_id, addr) in addrs.into_iter().filter_map(split_multiaddr) {
            connected.entry(peer_id).or_default().insert(addr);
        }
        for (peer_id, peer_state) in self.peers.iter_mut() {
            let connected_addrs = connected.get(peer_id).cloned().unwrap_or_default();
            // update connection state
            peer_state.connection_state = if connected_addrs.is_empty() {
                ConnectionState::Disconnected
            } else {
                ConnectionState::Connected
            };
            // make sure we got an entry for each of the connected addrs
            for connected_addr in &connected_addrs {
                peer_state.addresses.entry(connected_addr.clone()).or_default();
            }
            // update address state for each
            for (addr, addr_info) in peer_state.addresses.iter_mut() {
                addr_info
                    .state
                    .set(AddressState::from_connected(connected_addrs.contains(addr)));
            }
        }
    }

    pub fn apply_poll_parameters(&mut self, poll_parameters: &mut impl PollParameters) {
        let observed_addrs: BTreeSet<Multiaddr> = poll_parameters.external_addresses().map(|r| r.addr).collect();
        if observed_addrs != self.observed_addrs {
            info!("got new observed addrs {:?}", observed_addrs);
            self.observed_addrs = observed_addrs
        }
    }

    fn set_connection_state(&mut self, peer_id: PeerId, connection_state: ConnectionState) -> bool {
        debug!("set_connection_state peer:{} state:{:?}", peer_id, connection_state);
        let entry = self.peers.entry(peer_id).or_default();
        let result = entry.connection_state != connection_state;
        entry.connection_state = connection_state;
        result
    }

    fn set_address_state(&mut self, peer_id: PeerId, addr: Multiaddr, state: AddressState) {
        debug!("set_address_state peer:{} addr:{:?} state:{:?}", peer_id, addr, state);
        let peer_state = self.peers.entry(peer_id).or_default();
        let addr_info = peer_state.addresses.entry(addr).or_default();
        addr_info.state.set(state);
    }

    /// adds a new listen addr for a peer. Returns the multiaddr to be dialed
    pub fn add_listen_addr(
        &mut self,
        peer_id: PeerId,
        mut address: Multiaddr,
        provenance: AddressProvenance,
    ) -> Vec<Multiaddr> {
        if peer_id == self.peer_id {
            // don't dial ourselves. libp2p would ignore this anyway, but still...
            return Vec::new();
        }
        // strip peer id before we add it to our dicts, and log if anything is odd
        canonicalize_peer_address(&peer_id, &mut address);
        let entry = self.peers.entry(peer_id).or_default();
        //entry.connection_provenance = entry.connection_provenance.max_provenance(provenance);
        let info = entry.addresses.entry(address.clone()).or_default();
        info.provenance = info.provenance.max(provenance);
        // if this is a new peer, or a peer for which we have just discovered a new address,
        // return true so the outside knows to dial the peer.
        if entry.connection_state == ConnectionState::Disconnected {
            // this is the address with a possible peer id removed, so it is dialable by rust-libp2p
            vec![address]
        } else {
            Vec::new()
        }
    }

    fn remove_listen_addr(&mut self, peer_id: PeerId, address: &Multiaddr) -> Vec<Multiaddr> {
        if peer_id != self.peer_id {
            // strip peer id before we remove it to our dicts, and log if anything is odd
            // note that this is probably never necessary when running with rust-libp2p
            let mut address = address.clone();
            canonicalize_peer_address(&peer_id, &mut address);
            if let Some(entry) = self.peers.get_mut(&peer_id) {
                entry.addresses.remove(&address);
            }
        }
        Vec::new()
    }

    fn include_node_info(&mut self, info: NodeInfo) -> Vec<Multiaddr> {
        debug!("include_node_info {} {:?}", self.peer_id, info);
        let mut res = Vec::new();
        for (peer_id, addresses) in info.addresses {
            if peer_id == self.peer_id {
                continue;
            }
            let addresses = addresses
                .into_iter()
                .map(|mut address| {
                    canonicalize_peer_address(&peer_id, &mut address);
                    address
                })
                .collect::<BTreeSet<_>>();
            let entry = self.peers.entry(peer_id).or_default();
            // the info is assumed to be complete, so just keep addresses that are given in the update
            let to_remove = entry
                .addresses
                .iter()
                .filter(|(k, v)| {
                    // do not replace bootstrap node
                    // no not replace addrs that we are connected to
                    !v.state.is_connected() && v.provenance != AddressProvenance::Bootstrap && !addresses.contains(k)
                })
                .map(|(k, _)| k)
                .cloned()
                .collect::<Vec<_>>();
            for addr in to_remove.iter() {
                entry.addresses.remove(addr);
            }
            // make sure we have an entry for each address, but we can not know the connectivity state
            for address in addresses {
                if let Entry::Vacant(e) = entry.addresses.entry(address.clone()) {
                    e.insert(AddressInfo::default());
                    if entry.connection_state == ConnectionState::Disconnected {
                        res.push(address);
                    }
                }
            }
        }
        res
    }
}

fn canonicalize_peer_address(expected: &PeerId, address: &mut Multiaddr) {
    if let Some(stripped) = strip_peer_id(address) {
        if *expected == stripped {
            trace!("stripped peer id from multiaddr {} {}", expected, address);
        } else {
            error!(
                "stripped wrong peer id from multiaddr {} {} {}",
                expected, stripped, address
            );
        }
    }
}

fn split_multiaddr(mut addr: Multiaddr) -> Option<(PeerId, Multiaddr)> {
    if let Some(Protocol::P2p(peer)) = addr.pop() {
        if let Ok(peer_id) = PeerId::from_multihash(peer) {
            Some((peer_id, addr))
        } else {
            None
        }
    } else {
        None
    }
}

fn dialed_addr(cp: &ConnectedPoint) -> Option<&Multiaddr> {
    if let ConnectedPoint::Dialer { address } = cp {
        Some(address)
    } else {
        None
    }
}

/// The provenance of a given address, used e.g. for decision on GC.
/// Addresses provisioned via Bootstrap are never GC'ed,
/// while others are currently being GC'ed after GC_INTERVAL has lapsed
/// with the address in the Disconnected state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, PartialOrd, Ord)]
pub enum AddressProvenance {
    // The order of the variants defines the priority ordering, so please don’t reorder them!
    Swarm, // we have learned about the address from the Swarm, but we don't know exact provenance
    MDNS,
    Discovery, // currently we propose that Discovery is a more certain provenance than MDNS
    Bootstrap,
}

// the default case is being used for address provided by the Swarm
// (legacy interaction with go-ipfs, where we cannot be sure of exact provenance)
impl Default for AddressProvenance {
    fn default() -> Self {
        AddressProvenance::Swarm
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
}

impl Default for ConnectionState {
    fn default() -> Self {
        ConnectionState::Disconnected
    }
}

/// State of a known peer
#[derive(Clone, Debug, Default)]
pub struct PeerState {
    /// All known addresses of this peer, with some additional info.
    addresses: BTreeMap<Multiaddr, AddressInfo>,
    connection_state: ConnectionState,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(into = "AddressStateIo")]
#[allow(dead_code)]
pub enum AddressState {
    /// we have just learned about this address
    Initial,
    /// address is disconnected
    Disconnected {
        /// time since we are in this state
        since: Instant,
    },
    /// address is connected
    Connected {
        /// time since we are in this state
        since: Instant,
    },
}

impl AddressState {
    fn disconnected() -> Self {
        Self::Disconnected { since: Instant::now() }
    }

    fn connected() -> Self {
        Self::Connected { since: Instant::now() }
    }

    fn is_disconnected(&self) -> bool {
        matches!(self, AddressState::Disconnected { .. })
    }

    fn is_connected(&self) -> bool {
        matches!(self, AddressState::Connected { .. })
    }

    fn from_connected(connected: bool) -> Self {
        if connected {
            Self::connected()
        } else {
            Self::disconnected()
        }
    }

    /// sets the state, but keeps the time if the enum case is unchanged
    fn set(&mut self, value: Self) {
        match (&self, &value) {
            (AddressState::Disconnected { .. }, AddressState::Disconnected { .. }) => {}
            (AddressState::Connected { .. }, AddressState::Connected { .. }) => {}
            (AddressState::Initial, AddressState::Initial) => {}
            _ => *self = value,
        }
    }

    fn since(&self) -> Option<Instant> {
        match self {
            AddressState::Disconnected { since } => Some(*since),
            AddressState::Connected { since } => Some(*since),
            AddressState::Initial => None,
        }
    }
}

impl Default for AddressState {
    fn default() -> Self {
        Self::Initial
    }
}

impl cmp::Ord for AddressState {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        fn prio(x: &AddressState) -> u64 {
            match x {
                AddressState::Initial => 0,
                AddressState::Disconnected { .. } => 1,
                AddressState::Connected { .. } => 2,
            }
        }
        (prio(self), self.since()).cmp(&(prio(other), other.since()))
    }
}

impl cmp::PartialOrd for AddressState {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AddressInfo {
    state: AddressState,
    provenance: AddressProvenance,
}

impl AddressInfo {
    pub fn has_lapsed(&self, now: Instant) -> bool {
        self.state.is_disconnected()
            && self.provenance != AddressProvenance::Bootstrap
            && self
                .state
                .since()
                .map(|since| (now - since) > PRUNE_ADDRESS_AFTER) // can be later refactored to admit varying GC_INTERVAL values per provenance type
                .unwrap_or(false)
    }
}

impl Default for AddressInfo {
    fn default() -> Self {
        AddressInfo {
            state: AddressState::default(),
            provenance: AddressProvenance::default(),
        }
    }
}

pub(crate) struct DebugSwarmEvent<'a, T, E>(pub &'a SwarmEvent<T, E>);

impl<'a, T, E> fmt::Debug for DebugSwarmEvent<'a, T, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            SwarmEvent::NewListenAddr(address) => f.debug_struct("NewListenAddr").field("address", address).finish(),
            SwarmEvent::ExpiredListenAddr(address) => {
                f.debug_struct("ExpiredListenAddr").field("address", address).finish()
            }
            SwarmEvent::Dialing(peer_id) => f.debug_struct("Dialing").field("peer_id", peer_id).finish(),
            SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
            } => f
                .debug_struct("ConnectionEstablished")
                .field("peer_id", peer_id)
                .field("endpoint", endpoint)
                .field("num_established", num_established)
                .finish(),
            SwarmEvent::ConnectionClosed {
                peer_id,
                num_established,
                endpoint,
                ..
            } => f
                .debug_struct("ConnectionClosed")
                .field("peer_id", peer_id)
                .field("endpoint", endpoint)
                .field("num_established", num_established)
                .finish(),
            SwarmEvent::UnreachableAddr {
                peer_id,
                address,
                attempts_remaining,
                ..
            } => f
                .debug_struct("UnreachableAddr")
                .field("peer_id", peer_id)
                .field("address", address)
                .field("attempts_remaining", attempts_remaining)
                .finish(),
            SwarmEvent::IncomingConnection {
                local_addr,
                send_back_addr,
            } => f
                .debug_struct("IncomingConnection")
                .field("local_addr", local_addr)
                .field("send_back_addr", send_back_addr)
                .finish(),
            SwarmEvent::IncomingConnectionError {
                local_addr,
                send_back_addr,
                ..
            } => f
                .debug_struct("IncomingConnectionError")
                .field("local_addr", local_addr)
                .field("send_back_addr", send_back_addr)
                .finish(),
            SwarmEvent::UnknownPeerUnreachableAddr { address, .. } => f
                .debug_struct("UnknownPeerUnreachableAddr")
                .field("address", address)
                .finish(),
            SwarmEvent::BannedPeer { peer_id, endpoint } => f
                .debug_struct("BannedPeer")
                .field("peer_id", peer_id)
                .field("endpoint", endpoint)
                .finish(),
            SwarmEvent::ListenerClosed { addresses, reason } => f
                .debug_struct("ListenerClosed")
                .field("addresses", addresses)
                .field("reason", reason)
                .finish(),
            SwarmEvent::ListenerError { error } => f.debug_struct("ListenerError").field("error", error).finish(),
            SwarmEvent::Behaviour { .. } => f.debug_struct("Behaviour").finish(),
        }
    }
}

#[derive(Serialize)]
struct SwarmStateIo {
    peer_id: PeerIdIo,
    listen_addrs: BTreeSet<MultiaddrIo>,
    peers: BTreeMap<PeerIdIo, PeerStateIo>,
}

impl From<SwarmState> for SwarmStateIo {
    fn from(state: SwarmState) -> Self {
        Self {
            peer_id: state.peer_id.into(),
            listen_addrs: state.listen_addrs.into_iter().map(Into::into).collect(),
            peers: state.peers.into_iter().map(|(k, v)| (k.into(), v.into())).collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[allow(dead_code)]
pub enum AddressStateIo {
    /// address is disconnected
    Disconnected { since: u64 },
    /// address is connected
    Connected { since: u64 },
    ///
    Initial,
}

impl From<AddressState> for AddressStateIo {
    fn from(state: AddressState) -> Self {
        match state {
            AddressState::Initial => Self::Initial,
            AddressState::Disconnected { since } => Self::Disconnected {
                since: since.elapsed().as_secs(),
            },
            AddressState::Connected { since } => Self::Connected {
                since: since.elapsed().as_secs(),
            },
        }
    }
}

#[derive(Serialize)]
struct PeerStateIo {
    addresses: BTreeMap<MultiaddrIo, AddressInfo>,
    connection_state: ConnectionState,
}

impl From<PeerState> for PeerStateIo {
    fn from(state: PeerState) -> Self {
        Self {
            addresses: state.addresses.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            connection_state: state.connection_state,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::mutable_key_type)] // clippy bug #5812
    use super::*;
    use crate::discovery::protocol::*;
    use libp2p::core::ConnectedPoint;
    use maplit::btreeset;

    fn ma(text: &str) -> Multiaddr {
        text.parse().unwrap()
    }

    /// check that the state properly tracks its own addresses
    #[test]
    fn own_addrs() {
        let peer_self = PeerId::random();
        let addr_a = ma("/ip4/127.0.0.1/tcp/4001");
        let addr_b = ma("/ip4/8.8.8.8/udp/53");
        let mut state = SwarmState::new(peer_self);
        state.add_swarm_event::<(), ()>(&SwarmEvent::NewListenAddr(addr_a.clone()));
        assert_eq!(state.own_node_info().addresses[&peer_self], btreeset! {addr_a.clone()});
        state.add_swarm_event::<(), ()>(&SwarmEvent::NewListenAddr(addr_b.clone()));
        assert_eq!(
            state.own_node_info().addresses[&peer_self],
            btreeset! {addr_a.clone(), addr_b.clone()},
        );
        state.add_swarm_event::<(), ()>(&SwarmEvent::ExpiredListenAddr(addr_a.clone()));
        assert_eq!(state.own_node_info().addresses[&peer_self], btreeset! {addr_b.clone()});
        state.add_swarm_event::<(), ()>(&SwarmEvent::ExpiredListenAddr(addr_b.clone()));
        assert_eq!(state.own_node_info().addresses[&peer_self], btreeset! {});
    }

    /// check that the state properly tracks connection attempts to other peers
    #[test]
    fn connection_state() {
        let peer_self = PeerId::random();
        let peer_a = PeerId::random();
        let peer_b = PeerId::random();
        let mut state = SwarmState::new(peer_self);

        // unsuccessful connection attempt
        state.add_swarm_event::<(), ()>(&SwarmEvent::Dialing(peer_a));
        assert_eq!(state.peers[&peer_a].connection_state, ConnectionState::Connecting);
        state.add_swarm_event::<(), ()>(&SwarmEvent::ConnectionClosed {
            peer_id: peer_a,
            endpoint: ConnectedPoint::Dialer {
                address: ma("/ip4/1.2.3.4/tcp/4001"),
            },
            num_established: 0,
            cause: Some(libp2p::core::connection::ConnectionError::IO(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "that kinda sucks!",
            ))),
        });
        assert_eq!(state.peers[&peer_a].connection_state, ConnectionState::Disconnected);

        // successful connection attempt
        state.add_swarm_event::<(), ()>(&SwarmEvent::Dialing(peer_b));
        assert_eq!(state.peers[&peer_b].connection_state, ConnectionState::Connecting);
        state.add_swarm_event::<(), ()>(&SwarmEvent::ConnectionEstablished {
            peer_id: peer_b,
            num_established: std::num::NonZeroU32::new(1).unwrap(),
            endpoint: ConnectedPoint::Dialer {
                address: ma("/ip4/1.2.3.4/tcp/4001"),
            },
        });
        assert_eq!(state.peers[&peer_b].connection_state, ConnectionState::Connected);

        // check (dis)connected peers state
        assert_eq!(state.connected_peers(), vec![peer_b]);
        // a might be disconnected, but we still know that he lives
        assert_eq!(state.disconnected_peers(), vec![peer_a]);
    }

    #[test]
    fn discovery_msgs() {
        let peer_self = PeerId::random();
        let peer_a = PeerId::random();
        let peer_b = PeerId::random();
        let peer_c = PeerId::random();
        let addr_a = ma("/ip4/127.0.0.1/tcp/4001");
        let addr_b = ma("/ip4/8.8.8.8/udp/53");
        let addr_c = ma("/ip4/4.4.4.4/udp/53");
        let mut state = SwarmState::new(peer_self);

        // leared a new listen addr from another peer
        state.add_discovery_message(DiscoveryMessage::NewListenAddr(NewListenAddr {
            peer: peer_a,
            addr: addr_a.clone(),
        }));
        assert_eq!(state.addresses_of_peer(&peer_a), btreeset! { addr_a.clone() });

        // learned that said address is now expired
        state.add_discovery_message(DiscoveryMessage::ExpiredListenAddr(ExpiredListenAddr {
            peer: peer_a,
            addr: addr_a.clone(),
        }));
        assert_eq!(state.addresses_of_peer(&peer_a), btreeset! {});

        // leared a new listen addr from peer b
        state.add_discovery_message(DiscoveryMessage::NewListenAddr(NewListenAddr {
            peer: peer_b,
            addr: addr_c.clone(),
        }));
        // learned all new addrs of peer b via a node info msg
        state.add_discovery_message(DiscoveryMessage::NodeInfo(NodeInfo {
            addresses: btreemap! {
                peer_b => btreeset!{addr_a.clone(), addr_b.clone()},
            },
            stats: NodeStats::default(),
        }));
        assert_eq!(
            state.addresses_of_peer(&peer_b),
            btreeset! { addr_a.clone(), addr_b.clone() }
        );

        let addr_bs = ma("/dns4/demo-bootstrap.actyx.net/tcp/4001/p2p/QmUD1mA3Y8qSQB34HmgSNcxDss72UHW2kzQy7RdVstN2hH");
        let mut addr_bs_naked = addr_bs.clone();
        let peer_bs = if let Some(Protocol::P2p(mh)) = addr_bs_naked.pop() {
            PeerId::from_multihash(mh).unwrap()
        } else {
            panic!()
        };
        // check that a bootstrap addr is not being replaced by node info
        state.add_bootstrap(addr_bs.clone());
        state.add_discovery_message(DiscoveryMessage::NodeInfo(NodeInfo {
            addresses: btreemap! {
                peer_bs => btreeset!{addr_a.clone(), addr_b.clone()},
            },
            stats: NodeStats::default(),
        }));
        // addresses_of_peer must still contain the bs addr.
        assert_eq!(
            state.addresses_of_peer(&peer_bs),
            btreeset! { addr_a.clone(), addr_b.clone(), addr_bs_naked.clone() }
        );
        // check that a connected addr is not being replaced by node info
        state.add_discovery_message(DiscoveryMessage::NodeInfo(NodeInfo {
            addresses: btreemap! {
                peer_c => btreeset!{addr_c.clone()},
            },
            stats: NodeStats::default(),
        }));
        state.set_address_state(peer_c, addr_c.clone(), AddressState::connected());
        state.add_discovery_message(DiscoveryMessage::NodeInfo(NodeInfo {
            addresses: btreemap! {
                peer_c => btreeset!{addr_a.clone(), addr_b.clone()},
            },
            stats: NodeStats::default(),
        }));
        // addresses_of_peer must still contain the c addr, since it is connected.
        assert_eq!(
            state.addresses_of_peer(&peer_c),
            btreeset! { addr_a.clone(), addr_b.clone(), addr_c.clone() }
        );
    }
}
