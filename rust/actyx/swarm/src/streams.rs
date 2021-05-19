use crate::{AxStreamBuilder, Cid, Link, Tree};
use actyxos_sdk::{LamportTimestamp, NodeId, Offset, StreamId, StreamNr};
use ax_futures_util::stream::variable::Variable;
use fnv::FnvHashMap;
use futures::{
    future,
    stream::{BoxStream, Stream, StreamExt},
};
use std::{
    convert::{TryFrom, TryInto},
    ops::{Deref, DerefMut},
    sync::Arc,
};
use trees::{axtrees::AxTree, AxTreeHeader};

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
pub struct OwnStream {
    /// the stream number, just for convenience
    stream_nr: StreamNr,
    /// the builder, wrapped into an async mutex
    builder: tokio::sync::Mutex<AxStreamBuilder>,
    /// the latest published tree
    latest: Variable<Option<PublishedTree>>,
}

impl OwnStream {
    pub fn new(stream_nr: StreamNr, builder: AxStreamBuilder, latest: Option<PublishedTree>) -> Self {
        Self {
            stream_nr,
            builder: tokio::sync::Mutex::new(builder),
            latest: Variable::new(latest),
        }
    }

    pub fn tree_stream(&self) -> BoxStream<'static, Tree> {
        self.latest
            .new_projection(|x| x.as_ref().map(|x| x.tree.clone()).unwrap_or_default())
            .boxed()
    }

    /// The current root of the own stream
    ///
    /// Note that if you want to do something more complex with a tree, like transform it,
    /// using this will probably lead to a race condition.
    ///
    /// Use lock to get exclusive access to the tree in that case.
    pub fn snapshot(&self) -> Tree {
        self.latest
            .project(|x| x.as_ref().map(|x| x.tree.clone()).unwrap_or_default())
    }

    pub fn root(&self) -> Option<Cid> {
        self.latest.project(|x| x.as_ref().map(|x| Cid::from(x.root)))
    }

    /// Acquire an async lock to modify this stream
    pub async fn lock(&self) -> OwnStreamGuard<'_> {
        OwnStreamGuard(self, self.builder.lock().await)
    }
}

pub struct OwnStreamGuard<'a>(&'a OwnStream, tokio::sync::MutexGuard<'a, AxStreamBuilder>);

impl<'a> OwnStreamGuard<'a> {
    pub fn latest(&self) -> &Variable<Option<PublishedTree>> {
        &self.0.latest
    }
    pub fn stream_nr(&self) -> StreamNr {
        self.0.stream_nr
    }
}

impl<'a> Deref for OwnStreamGuard<'a> {
    type Target = AxStreamBuilder;

    fn deref(&self) -> &AxStreamBuilder {
        self.1.deref()
    }
}

impl<'a> DerefMut for OwnStreamGuard<'a> {
    fn deref_mut(&mut self) -> &mut AxStreamBuilder {
        self.1.deref_mut()
    }
}

#[derive(Debug, Default)]
pub struct RemoteNodeInner {
    pub last_seen: Variable<(LamportTimestamp, Offset)>,
    pub streams: FnvHashMap<StreamNr, Arc<ReplicatedStream>>,
}

/// Data for a single replicated stream, mutable state + constant data
#[derive(Debug)]
pub struct ReplicatedStream {
    // this is an option to cover the situation where we learn of a remote stream
    // but have not yet validated it
    validated: Variable<Option<PublishedTree>>,
    // stream of incoming roots
    incoming: Variable<Option<Link>>,
    latest_seen: Variable<Option<(LamportTimestamp, Offset)>>,
}

/// Trees are published including a tree header.
#[derive(Debug)]
pub struct PublishedTree {
    /// hash of the tree header
    root: Link,
    /// the tree header
    header: AxTreeHeader,
    /// the actual tree
    tree: AxTree,
}

impl PublishedTree {
    pub fn new(root: Link, header: AxTreeHeader, tree: AxTree) -> Self {
        Self { root, header, tree }
    }
}

impl ReplicatedStream {
    pub fn new(state: Option<PublishedTree>) -> Self {
        Self {
            validated: Variable::new(state),
            incoming: Variable::default(),
            latest_seen: Variable::default(),
        }
    }

    /// set the latest validated root
    pub fn set_latest(&self, value: PublishedTree) {
        self.validated.set(Some(value));
    }

    /// root of the tree. This is the hash of the header.
    pub fn root(&self) -> Option<Cid> {
        self.validated.project(|x| x.as_ref().map(|x| Cid::from(x.root)))
    }

    /// lamport of the header and count of the last validated tree.
    /// Will default to (0, 0) if there is no header yet.
    pub fn validated_tree_counters(&self) -> (LamportTimestamp, u64) {
        self.validated.project(|x| {
            x.as_ref()
                .map(|x| (x.header.lamport, x.tree.count()))
                .unwrap_or_default()
        })
    }

    /// set the latest incoming root. This will trigger validation
    pub fn set_incoming(&self, value: Link) {
        self.incoming.set(Some(value));
    }

    pub fn tree_stream(&self) -> BoxStream<'static, Tree> {
        self.validated
            .new_projection(|x| x.as_ref().map(|x| x.tree.clone()).unwrap_or_default())
            .boxed()
    }

    pub fn incoming_root_stream(&self) -> impl Stream<Item = Link> {
        self.incoming.new_observer().filter_map(future::ready)
    }

    pub fn latest_seen(&self) -> &Variable<Option<(LamportTimestamp, Offset)>> {
        &self.latest_seen
    }
}
