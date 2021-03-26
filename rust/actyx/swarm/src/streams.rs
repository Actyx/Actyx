use crate::{AxTreeExt, Forest, Link, Tree};
use actyxos_sdk::{LamportTimestamp, NodeId, Offset, StreamId, StreamNr};
use ax_futures_util::stream::variable::{self, Variable};
use fnv::FnvHashMap;
use futures::{
    channel::mpsc,
    future,
    stream::{Stream, StreamExt},
};
use std::collections::BTreeMap;
use std::convert::{TryFrom, TryInto};
use std::sync::Arc;
use trees::{RootMap, RootMapEntry};

const PREFIX: u8 = b'S';

/// Helper to store a stream id as an alias name in sqlite
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StreamAlias([u8; 41]);

impl AsRef<[u8]> for StreamAlias {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl TryFrom<&[u8]> for StreamAlias {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(value.len() == 41, "StreamAlias must be 41 bytes");
        anyhow::ensure!(value[0] == PREFIX, "Prefix must be the letter S");
        let node_id = NodeId::from_bytes(&value[1..33])?;
        let stream_nr = u64::from_be_bytes(value[33..41].try_into()?).try_into()?;
        Ok(node_id.stream(stream_nr).into())
    }
}

impl From<StreamId> for StreamAlias {
    fn from(value: StreamId) -> Self {
        let mut result = [0; 41];
        result[0] = PREFIX;
        result[1..33].copy_from_slice(value.node_id().as_ref());
        result[33..41].copy_from_slice(&u64::from(value.stream_nr()).to_be_bytes());
        StreamAlias(result)
    }
}

impl TryFrom<StreamAlias> for StreamId {
    type Error = anyhow::Error;

    fn try_from(value: StreamAlias) -> anyhow::Result<Self> {
        let node_id = NodeId::from_bytes(&value.0[1..33])?;
        let stream_nr = u64::from_be_bytes(value.0[33..41].try_into()?).try_into()?;
        Ok(node_id.stream(stream_nr))
    }
}

/// Data for a single own stream, mutable state + constant data
#[derive(Debug)]
pub struct OwnStreamInner {
    pub forest: Forest,
    pub sequencer: tokio::sync::Mutex<()>,
    pub tree: Variable<Tree>,
    pub latest_seen: Variable<Option<(LamportTimestamp, Offset)>>,
}

impl OwnStreamInner {
    pub fn new(forest: Forest) -> Self {
        Self {
            forest,
            sequencer: tokio::sync::Mutex::new(()),
            tree: Variable::default(),
            latest_seen: Variable::default(),
        }
    }

    pub fn latest(&self) -> Tree {
        self.tree.get_cloned()
    }

    pub fn offset(&self) -> Option<Offset> {
        let count = self.tree.project(|tree| tree.count());
        if count == 0 {
            None
        } else {
            Some(Offset::try_from(count - 1).unwrap())
        }
    }

    pub fn set_latest(&self, value: Tree) {
        self.tree.set(value)
    }

    pub fn tree_stream(&self) -> variable::Observer<Tree> {
        self.tree.new_observer()
    }
}

#[derive(Debug, Default)]
pub struct RemoteNodeInner {
    pub last_seen: Variable<(LamportTimestamp, Offset)>,
    pub streams: FnvHashMap<StreamNr, Arc<ReplicatedStreamInner>>,
}

/// Data for a single replicated stream, mutable state + constant data
#[derive(Debug)]
pub struct ReplicatedStreamInner {
    pub forest: Forest,
    pub validated: Variable<Tree>,
    pub incoming: Variable<Option<Link>>,
    pub latest_seen: Variable<Option<(LamportTimestamp, Offset)>>,
}

impl ReplicatedStreamInner {
    pub fn new(forest: Forest) -> Self {
        Self {
            forest,
            validated: Variable::default(),
            incoming: Variable::default(),
            latest_seen: Variable::default(),
        }
    }

    pub fn root_map_entry(&self) -> Option<RootMapEntry> {
        self.validated.project(|tree| {
            let lamport = tree.last_lamport();
            tree.link().map(|link| RootMapEntry::new(&link.into(), lamport))
        })
    }

    /// set the latest validated root
    pub fn set_latest(&self, value: Tree) {
        self.validated.set(value);
    }

    /// latest validated root
    pub fn validated(&self) -> Tree {
        self.validated.get_cloned()
    }

    /// set the latest incoming root. This will trigger validation
    pub fn set_incoming(&self, value: Link) {
        self.incoming.set(Some(value));
    }

    pub fn tree_stream(&self) -> variable::Observer<Tree> {
        self.validated.new_observer()
    }

    pub fn incoming_root_stream(&self) -> impl Stream<Item = Link> {
        self.incoming.new_observer().filter_map(future::ready)
    }
}

// maps of own and replicated streams, plus notification mechanism when new streams are created
#[derive(Default)]
pub struct StreamMaps {
    pub own_streams: BTreeMap<StreamNr, Arc<OwnStreamInner>>,
    pub remote_nodes: BTreeMap<NodeId, RemoteNodeInner>,
    pub known_streams: Vec<mpsc::UnboundedSender<StreamId>>,
}

impl StreamMaps {
    pub fn publish_new_stream_id(&mut self, stream_id: StreamId) {
        self.known_streams
            .retain(|sender| sender.unbounded_send(stream_id).is_ok())
    }

    pub fn current_stream_ids(&self, node_id: NodeId) -> impl Iterator<Item = StreamId> + '_ {
        let own_stream_ids = self.own_streams.keys().map(move |stream_id| node_id.stream(*stream_id));
        let replicated_stream_ids = self.remote_nodes.iter().flat_map(|(node_id, node_info)| {
            node_info
                .streams
                .keys()
                .map(move |stream_nr| node_id.stream(*stream_nr))
        });
        own_stream_ids.chain(replicated_stream_ids)
    }

    /// Get a complete root map from both own and replicated streams
    pub fn root_map(&self, own_node_id: NodeId) -> RootMap {
        let own = self.own_streams.iter().filter_map(|(stream_nr, inner)| {
            let (link, lamport) = inner.tree.project(|tree| (tree.link(), tree.last_lamport()));
            let stream_id = own_node_id.stream(*stream_nr);
            link.map(|link| (stream_id, RootMapEntry::new(&link.into(), lamport)))
        });

        let other = self.remote_nodes.iter().flat_map(|(node_id, remote_node)| {
            remote_node.streams.iter().filter_map(move |(stream_nr, inner)| {
                let stream_id = node_id.stream(*stream_nr);
                inner.root_map_entry().map(|e| (stream_id, e))
            })
        });
        RootMap(own.chain(other).collect())
    }

    pub fn get_or_create_remote_node(&mut self, node_id: NodeId) -> &mut RemoteNodeInner {
        self.remote_nodes.entry(node_id).or_insert_with(|| {
            tracing::debug!("learned of new node {}", node_id);
            Default::default()
        })
    }
}
