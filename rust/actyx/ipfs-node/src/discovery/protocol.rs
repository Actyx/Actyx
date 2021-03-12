//! The protocol of the gossipsub based connectivity mechanism
//!
//! The discovery protocol is intended to be somewhat stable. We won't completely
//! change it without having backwards compatibility. So tapping into the discovery
//! protcol to get an overview over the connectivity state of a swarm would be viable.
//!
//! # Messages
//!
//! ## NodeInfo
//!
//! Complete information about the addresses under which a node can be reached, as well as
//! some very high level stats about the node connectivity state.
//!
//! Just parsing this will give a pretty good idea of the swarm state. The other messages
//! in the protocol are just to reduce latency. E.g. if a node gains a new listen addr, this
//! information will eventually make it into its node info.
//!
//! ### Example
//!
//!```json
//!{
//!  "addresses": {
//!     "12D3KooWHhbGYPu4kXfp3iNJq54RNFHA8SZk29vgNQwQ5zYJL5x1": [
//!      "/ip4/127.0.0.1/tcp/4001",
//!      "/ip4/172.17.0.2/tcp/4001",
//!      "/ip4/172.26.0.1/tcp/4001",
//!      "/ip4/193.159.75.75/tcp/4001"
//!    ]
//!  },
//!  "stats": {
//!    "connected_peers": 1,
//!    "known_peers": 19
//!  },
//!  "type": "NodeInfo"
//!}
//!```
//! The node `12D3KooWHhbGYPu4kXfp3iNJq54RNFHA8SZk29vgNQwQ5zYJL5x1` is reachable via the
//! addresses `/ip4/127.0.0.1/tcp/4001`, `/ip4/172.17.0.2/tcp/4001`, `/ip4/172.26.0.1/tcp/4001`, `/ip4/193.159.75.75/tcp/4001`.
//!
//! It currently knows about 19 peers and is connected to 1 peer.
//!
//! Currently, the message will contain information about a single node, the sender of the message.
//! In the future we might extend it so that a node will also gossip about what it knows about other nodes.
//!
//! ## NewListenAddr
//!
//! A node has gained a new listen addr. This information will end up in the NodeInfo eventually.
//!
//! ## ExpiredListenAddr
//!
//! A listen addr of a node is no longer available.  This information will end up in the NodeInfo eventually.
#![allow(clippy::mutable_key_type)] // clippy bug #5812
#![allow(clippy::redundant_clone)]
use crate::discovery::formats::{MultiaddrIo, PeerIdIo};
use derive_more::From;
use libp2p::{Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use util::serde_util::{from_json_or_cbor_slice, JsonCborDeserializeError};

// --- internal model starts here ---

/// The discovery protocol
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum DiscoveryMessage {
    /// complete address information for one or more nodes
    NodeInfo(NodeInfo),
    NewListenAddr(NewListenAddr),
    ExpiredListenAddr(ExpiredListenAddr),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PublishMode {
    // publish as CBOR. More compact.
    #[allow(dead_code)]
    Cbor,
    // publish as json. Peer ids and multiaddrs will be more easy to debug.
    Json,
}

/// Information about the addresses of one or more nodes.
///
/// The information is assumed to be complete, so the list of addresses for a node should completely
/// replace the previous list of addresses.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "NodeInfoIo", into = "NodeInfoIo")]
pub struct NodeInfo {
    pub addresses: BTreeMap<PeerId, BTreeSet<Multiaddr>>,
    pub stats: NodeStats,
}

/// a single node has a new listen addr
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "NewListenAddrIo", into = "NewListenAddrIo")]
pub struct NewListenAddr {
    pub peer: PeerId,
    pub addr: Multiaddr,
}

/// a single node has an expired listen addr
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "ExpiredListenAddrIo", into = "ExpiredListenAddrIo")]
pub struct ExpiredListenAddr {
    pub peer: PeerId,
    pub addr: Multiaddr,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NodeStats {
    pub known_peers: u64,
    pub connected_peers: u64,
}

impl DiscoveryMessage {
    pub fn to_bytes(&self, mode: PublishMode) -> Vec<u8> {
        match mode {
            PublishMode::Cbor => serde_cbor::to_vec(self).unwrap(),
            PublishMode::Json => serde_json::to_vec(self).unwrap(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> std::result::Result<DiscoveryMessage, JsonCborDeserializeError> {
        let result = from_json_or_cbor_slice::<DiscoveryMessage>(bytes);
        if let Err(cause) = &result {
            tracing::warn!("unable to deserialize discovery message: {}", cause);
        }
        result
    }
}

// --- boilerplate starts here ---

#[derive(Serialize, Deserialize)]
struct NewListenAddrIo {
    peer: PeerIdIo,
    addr: MultiaddrIo,
}

impl From<NewListenAddr> for NewListenAddrIo {
    fn from(value: NewListenAddr) -> Self {
        Self {
            peer: value.peer.into(),
            addr: value.addr.into(),
        }
    }
}

impl From<NewListenAddrIo> for NewListenAddr {
    fn from(value: NewListenAddrIo) -> Self {
        Self {
            peer: value.peer.into(),
            addr: value.addr.into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ExpiredListenAddrIo {
    peer: PeerIdIo,
    addr: MultiaddrIo,
}

impl From<ExpiredListenAddr> for ExpiredListenAddrIo {
    fn from(value: ExpiredListenAddr) -> Self {
        Self {
            peer: value.peer.into(),
            addr: value.addr.into(),
        }
    }
}

impl From<ExpiredListenAddrIo> for ExpiredListenAddr {
    fn from(value: ExpiredListenAddrIo) -> Self {
        Self {
            peer: value.peer.into(),
            addr: value.addr.into(),
        }
    }
}

/// node stats is so simple that it can be directly serialized
type NodeStatsIo = NodeStats;

#[derive(Serialize, Deserialize)]
struct NodeInfoIo {
    addresses: BTreeMap<PeerIdIo, Vec<MultiaddrIo>>,
    #[serde(default)]
    stats: NodeStatsIo,
}

impl From<NodeInfo> for NodeInfoIo {
    fn from(info: NodeInfo) -> Self {
        let addresses = info
            .addresses
            .into_iter()
            .map(|(k, vs)| {
                let k = PeerIdIo(k);
                let vs = vs.into_iter().map(MultiaddrIo).collect::<Vec<_>>();
                (k, vs)
            })
            .collect::<BTreeMap<_, _>>();
        Self {
            addresses,
            stats: info.stats,
        }
    }
}

impl From<NodeInfoIo> for NodeInfo {
    fn from(info: NodeInfoIo) -> Self {
        let addresses = info
            .addresses
            .into_iter()
            .map(|(k, v)| {
                let k = k.0;
                let v = v.into_iter().map(|e| e.0).collect::<BTreeSet<_>>();
                (k, v)
            })
            .collect::<BTreeMap<_, _>>();
        NodeInfo {
            addresses,
            stats: info.stats,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::mutable_key_type)] // clippy bug #5812
    use super::*;
    use libipld::cid::Cid;
    use libipld::multihash::Multihash;
    use maplit::{btreemap, btreeset};
    use serde_json::json;
    use std::convert::TryFrom;

    fn ma(text: &str) -> Multiaddr {
        text.parse().unwrap()
    }

    fn mh(text: &str) -> Multihash {
        *Cid::try_from(text).unwrap().hash()
    }

    fn p(text: &str) -> PeerId {
        PeerId::from_multihash(mh(text)).unwrap()
    }

    /// testing json => typed msgs => json roundtrip with handcrafted json msgs
    #[test]
    fn protocol_json() {
        let wire_data = json! {[
            {
                "type": "NodeInfo",
                "addresses": {
                    "Qmf4R1M1PHYdWy5i1HriSq44SUc6LbBcSKf3ZS7WDq4vNu": ["/ip4/8.8.8.8/udp/53", "/ip4/4.4.4.4/udp/53"]
                },
                "stats": {
                    "known_peers": 2,
                    "connected_peers": 1,
                }
            },
            {
                "type": "NewListenAddr",
                "peer": "Qmf4R1M1PHYdWy5i1HriSq44SUc6LbBcSKf3ZS7WDq4vNu",
                "addr": "/ip4/127.0.0.1/tcp/4001",
            },
            {
                "type": "ExpiredListenAddr",
                "peer": "Qmf4R1M1PHYdWy5i1HriSq44SUc6LbBcSKf3ZS7WDq4vNu",
                "addr": "/ip4/8.8.8.8/udp/53",
            },
        ]};
        let peer_id = p("Qmf4R1M1PHYdWy5i1HriSq44SUc6LbBcSKf3ZS7WDq4vNu");
        let msgs: Vec<DiscoveryMessage> = serde_json::from_value(wire_data.clone()).unwrap();
        let expected = vec![
            DiscoveryMessage::NodeInfo(NodeInfo {
                addresses: btreemap! { peer_id => btreeset!{
                    ma("/ip4/8.8.8.8/udp/53"), ma("/ip4/4.4.4.4/udp/53")
                }},
                stats: NodeStats {
                    known_peers: 2,
                    connected_peers: 1,
                },
            }),
            DiscoveryMessage::NewListenAddr(NewListenAddr {
                peer: peer_id,
                addr: ma("/ip4/127.0.0.1/tcp/4001"),
            }),
            DiscoveryMessage::ExpiredListenAddr(ExpiredListenAddr {
                peer: peer_id,
                addr: ma("/ip4/8.8.8.8/udp/53"),
            }),
        ];
        assert_eq!(msgs, expected);
        // serialization is no longer stable since we use a HashSet somewhere. Enable this again
        // once we got enough Ord instances upstream to use a BTreeSet again.
        // let json2 = serde_json::to_value(expected).unwrap();
        // assert_eq!(json2, wire_data);
    }

    /// testing json => typed msgs => json roundtrip with handcrafted json msgs
    ///
    /// compat when stats is not there
    #[test]
    fn protocol_json_compat() {
        let wire_data = json! {[
            {
                "type": "NodeInfo",
                "addresses": {
                    "Qmf4R1M1PHYdWy5i1HriSq44SUc6LbBcSKf3ZS7WDq4vNu": ["/ip4/8.8.8.8/udp/53", "/ip4/4.4.4.4/udp/53"]
                },
            },
        ]};
        let peer_id = p("Qmf4R1M1PHYdWy5i1HriSq44SUc6LbBcSKf3ZS7WDq4vNu");
        let msgs: Vec<DiscoveryMessage> = serde_json::from_value(wire_data.clone()).unwrap();
        let expected = vec![DiscoveryMessage::NodeInfo(NodeInfo {
            addresses: btreemap! { peer_id => btreeset!{
                ma("/ip4/8.8.8.8/udp/53"), ma("/ip4/4.4.4.4/udp/53")
            }},
            stats: NodeStats::default(),
        })];
        assert_eq!(msgs, expected);
    }

    /// testing typed msgs => cbor => typed msgs roundtrip
    ///
    /// this does not check anything in particular about the cbor except that it properly roundtrips
    #[test]
    fn cbor_roundtrip() {
        let peer_id = p("Qmf4R1M1PHYdWy5i1HriSq44SUc6LbBcSKf3ZS7WDq4vNu");
        let expected = vec![
            DiscoveryMessage::NodeInfo(NodeInfo {
                addresses: btreemap! { peer_id => btreeset!{
                    ma("/ip4/8.8.8.8/udp/53"), ma("/ip4/4.4.4.4/udp/53")
                }},
                stats: NodeStats {
                    known_peers: 2,
                    connected_peers: 1,
                },
            }),
            DiscoveryMessage::NewListenAddr(NewListenAddr {
                peer: peer_id,
                addr: ma("/ip4/127.0.0.1/tcp/4001"),
            }),
            DiscoveryMessage::ExpiredListenAddr(ExpiredListenAddr {
                peer: peer_id,
                addr: ma("/ip4/8.8.8.8/udp/53"),
            }),
        ];
        let buffer = serde_cbor::to_vec(&expected).unwrap();
        let actual: Vec<DiscoveryMessage> = serde_cbor::from_slice(&buffer).unwrap();
        assert_eq!(expected, actual);
    }
}
