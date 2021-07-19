use crate::{AxStreamBuilder, Cid, Link, RootSource, Tree};
use actyx_sdk::{LamportTimestamp, NodeId, Offset, Payload, StreamId, StreamNr};
use ax_futures_util::stream::variable::Variable;
use banyan::StreamTransaction;
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
use trees::{axtrees::AxTrees, AxTree, AxTreeHeader};

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

    pub fn published_tree(&self) -> Option<PublishedTree> {
        self.latest.get_cloned()
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

    pub fn transaction(&mut self) -> StreamTransaction<'_, AxTrees, Payload> {
        self.1.transaction()
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
    incoming: Variable<Option<(Link, RootSource)>>,
    latest_seen: Variable<Option<(LamportTimestamp, Offset)>>,
}

/// Trees are published including a tree header.
#[derive(Debug, Clone)]
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

    pub fn offset(&self) -> Offset {
        let offset = self.tree.count().checked_sub(1).expect("published tree empty");
        Offset::try_from(offset).expect("invalid offset")
    }

    pub fn root(&self) -> Link {
        self.root
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

    pub fn latest(&self) -> Option<PublishedTree> {
        self.validated.get_cloned()
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

    /// set the latest incoming root
    ///
    /// This will trigger validation if the `source` has sufficient priority compared to the current link.
    /// The recipient will have to call `downgrade()` after processing to ensure that later updates for
    /// new roots will be accepted.
    pub fn set_incoming(&self, value: Link, source: RootSource) {
        self.incoming.transform_mut(|x| match x {
            Some((l, s)) if *s > source || *s == source && *l == value => false,
            _ => {
                x.replace((value, source));
                true
            }
        });
    }

    /// Dial down the priority of the stored value to the minimum to allow later updates from any source
    pub fn downgrade(&self, link: Link, error: bool) {
        self.incoming.transform_mut(|x| {
            match x {
                Some((l, s)) if *l == link => {
                    if error {
                        x.take();
                    } else {
                        *s = RootSource::RootMap;
                    }
                }
                _ => {}
            }
            false
        });
    }

    pub fn tree_stream(&self) -> BoxStream<'static, Tree> {
        self.validated
            .new_projection(|x| x.as_ref().map(|x| x.tree.clone()).unwrap_or_default())
            .boxed()
    }

    pub fn incoming_root_stream(&self) -> impl Stream<Item = (Link, RootSource)> {
        self.incoming.new_observer().filter_map(future::ready)
    }
}
