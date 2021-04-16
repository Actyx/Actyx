//! #PubSub formats
//!
//! This module contains the formats that are sent on the IPFS pubsub topic of an installation.

use super::{ConsNode, FromStr, LamportTimestamp, Timestamp};
use actyxos_sdk::{NodeId, StreamId};
use libipld::{Cid, DagCbor, Link};
use serde::{Deserialize, Serialize};
use serde_json::Error;
use std::collections::btree_map::BTreeMap;
use std::fmt::{Display, Formatter};

/// A gossiped message that contains a specific node's view on the root nodes of
/// all other streams it knows about.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Eq, DagCbor)]
pub struct PublishHeartbeat {
    pub node: NodeId,
    pub timestamp: Timestamp,
    pub lamport: LamportTimestamp,
    pub roots: RootMap,
}

impl FromStr for PublishHeartbeat {
    type Err = Error;

    fn from_str(json: &str) -> Result<PublishHeartbeat, Error> {
        serde_json::from_str(json)
    }
}

impl Display for PublishHeartbeat {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let json = serde_json::to_string(self).unwrap();
        f.write_str(&*json)
    }
}

/// Represents the last known "head" of a given stream
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, DagCbor)]
pub struct RootMapEntry {
    #[serde(with = "serde_with::rust::display_fromstr")]
    pub cid: Cid,
    pub lamport: LamportTimestamp,
}

impl RootMapEntry {
    pub fn new(cid: &Cid, lamport: LamportTimestamp) -> RootMapEntry {
        RootMapEntry { cid: *cid, lamport }
    }

    pub fn root_link(&self) -> Link<ConsNode> {
        self.cid.into()
    }
}

impl FromStr for RootMapEntry {
    type Err = Error;

    fn from_str(json: &str) -> Result<RootMapEntry, Error> {
        serde_json::from_str(json)
    }
}

impl Display for RootMapEntry {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        let json = serde_json::to_string(self).unwrap();
        f.write_str(&*json)
    }
}

/// A collection of the last known roots of a known set of clients and their last
/// known lamports.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, DagCbor)]
pub struct RootMap(pub BTreeMap<StreamId, RootMapEntry>);

impl RootMap {
    /// Creates an empty root map, i.e. one with no known streams.
    pub fn empty() -> RootMap {
        RootMap(BTreeMap::new())
    }

    pub fn remove(&mut self, sid: StreamId) -> Option<RootMapEntry> {
        self.0.remove(&sid)
    }
}
