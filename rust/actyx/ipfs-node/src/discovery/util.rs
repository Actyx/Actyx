//! utilities to interact with go-ipfs config
use libp2p::{multiaddr::Protocol, Multiaddr, PeerId};

/// for a multiaddr that ends with a peer id, this strips this suffix.
/// Rust-libp2p only supports dialing to an address without providing the peer id.
pub fn strip_peer_id(addr: &mut Multiaddr) -> Option<PeerId> {
    let last = addr.pop();
    match last {
        Some(Protocol::P2p(peer_id)) => PeerId::from_multihash(peer_id).ok(),
        Some(other) => {
            addr.push(other);
            None
        }
        _ => None,
    }
}
