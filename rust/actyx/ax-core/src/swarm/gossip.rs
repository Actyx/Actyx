use crate::{
    ax_futures_util::stream::ready_iter,
    swarm::{
        gossip_protocol::{GossipMessage, RootMap, RootUpdate},
        BanyanStore, Ipfs, Link, RootPath, RootSource,
    },
};
use acto::ActoRef;
use anyhow::Result;
use ax_types::{LamportTimestamp, NodeId, Offset, StreamNr, Timestamp};
use cbor_data::{
    codec::{CodecError, ReadCbor, WriteCbor},
    Cbor, CborBuilder,
};
use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    prelude::*,
};
use ipfs_embed::{GossipEvent, PeerId};
use libipld::Cid;
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::TryFrom,
    time::Duration,
};

const MAX_BROADCAST_BYTES: usize = 1_000_000;

/// Update when we have rewritten a tree
#[derive(Debug)]
struct PublishUpdate {
    stream: StreamNr,
    root: Link,
    links: BTreeSet<Link>,
    lamport: LamportTimestamp,
    offset: Offset,
}

pub struct Gossip {
    tx: UnboundedSender<PublishUpdate>,
    publish_handle: tokio::task::JoinHandle<()>,
}

impl Gossip {
    pub fn new(
        mut ipfs: Ipfs,
        node_id: NodeId,
        topic: String,
        enable_fast_path: bool,
        enable_slow_path: bool,
        swarm_observer: ActoRef<(PeerId, GossipMessage)>,
    ) -> Self {
        let (tx, mut rx) = unbounded::<PublishUpdate>();
        let publish_task = async move {
            let mut cbor_scratch = Vec::new();

            while let Some(updates) = ready_iter(&mut rx).await {
                // drain the channel and only publish the latest update per stream
                let updates = updates.map(|up| (up.stream, up)).collect::<BTreeMap<_, _>>();

                for (_, update) in updates {
                    let _s = tracing::trace_span!("publishing", stream = %update.stream);
                    let _s = _s.enter();
                    let time = Timestamp::now();
                    let lamport = update.lamport;
                    let offset = update.offset;
                    let root = Cid::from(update.root);
                    let stream = node_id.stream(update.stream);
                    let mut size = 0;
                    let mut blocks = Vec::with_capacity(100);
                    for link in update.links {
                        let cid = Cid::from(link);
                        if let Ok(block) = ipfs.get(&cid) {
                            if size + block.data().len() > MAX_BROADCAST_BYTES {
                                break;
                            } else {
                                size += block.data().len();
                                blocks.push(block);
                            }
                        }
                    }
                    tracing::trace!(bytes = size, blocks = blocks.len());

                    swarm_observer.send((
                        ipfs.local_peer_id(),
                        GossipMessage::RootUpdate(RootUpdate {
                            stream,
                            root,
                            blocks: vec![],
                            lamport,
                            time,
                            offset: Some(offset),
                        }),
                    ));

                    if enable_fast_path {
                        let root_update = RootUpdate {
                            stream,
                            root,
                            blocks,
                            lamport,
                            time,
                            offset: Some(offset),
                        };
                        let blob = GossipMessage::RootUpdate(root_update)
                            .write_cbor(CborBuilder::with_scratch_space(&mut cbor_scratch))
                            .into_vec();
                        tracing::trace!("broadcast_blob {} {}", stream, blob.len());
                        if let Err(err) = ipfs.broadcast(topic.clone(), blob).await {
                            tracing::error!("broadcast failed: {}", err);
                        }
                    }

                    if enable_slow_path {
                        // slow path doesn't include blocks to prevent loading the network with
                        // duplicate data. peers that receive a root update will use bitswap to
                        // find the blocks they are missing.
                        let root_update = RootUpdate {
                            root,
                            stream,
                            lamport,
                            time,
                            blocks: Default::default(),
                            offset: Some(offset),
                        };
                        let blob = GossipMessage::RootUpdate(root_update)
                            .write_cbor(CborBuilder::with_scratch_space(&mut cbor_scratch))
                            .into_vec();
                        tracing::trace!(%stream, %topic, "publish_blob len {}", blob.len());
                        if let Err(err) = ipfs.publish(topic.clone(), blob).await {
                            tracing::error!(%stream, %topic, "publish failed: {}", err);
                        }
                    }
                }
            }
            tracing::error!("gossip loop stopped, live updates won’t work anymore");
        };
        Self {
            tx,
            publish_handle: tokio::spawn(publish_task),
        }
    }

    pub fn publish(
        &self,
        stream: StreamNr,
        root: Link,
        links: BTreeSet<Link>,
        lamport: LamportTimestamp,
        offset: Offset,
    ) -> Result<()> {
        self.tx.unbounded_send(PublishUpdate {
            stream,
            root,
            links,
            lamport,
            offset,
        })?;
        Ok(())
    }

    pub fn publish_root_map(
        &self,
        store: BanyanStore,
        topic: String,
        interval: Duration,
        swarm_observer: ActoRef<(PeerId, GossipMessage)>,
    ) -> impl Future<Output = ()> {
        let mut ipfs = store.ipfs().clone();
        async move {
            let mut cbor_scratch = Vec::new();
            loop {
                tokio::time::sleep(interval).await;
                let _s = tracing::trace_span!("publish_root_map");
                let _s = _s.enter();
                let guard = store.lock();
                let root_map = guard.root_map();
                let lamport = guard.data.lamport.get();
                drop(guard);

                let n_entries = root_map.len();
                let mut offsets = Vec::with_capacity(n_entries);
                let entries = root_map
                    .into_iter()
                    .map(|(stream, (root, offset, lamport))| {
                        offsets.push((offset, lamport));
                        (stream, root)
                    })
                    .collect();

                let time = Timestamp::now();
                let msg = GossipMessage::RootMap(RootMap {
                    entries,
                    offsets,
                    lamport,
                    time,
                });
                swarm_observer.send((ipfs.local_peer_id(), msg.clone()));
                let blob = msg
                    .write_cbor(CborBuilder::with_scratch_space(&mut cbor_scratch))
                    .into_vec();
                if let Err(err) = ipfs.publish(topic.clone(), blob).await {
                    tracing::error!("publish root map failed: {}", err);
                } else {
                    tracing::debug!("published {} entries at lamport {}", n_entries, lamport,);
                }
            }
        }
    }

    pub async fn ingest(
        store: BanyanStore,
        topic: String,
        swarm_observer: ActoRef<(PeerId, GossipMessage)>,
    ) -> Result<impl Future<Output = ()>> {
        let mut ipfs = store.ipfs().clone();
        let mut subscription = ipfs.subscribe(topic.clone()).await?;
        Ok(async move {
            while let Some(event) = subscription.next().await {
                let (peer_id, message) = if let GossipEvent::Message(sender, message) = event {
                    (sender, message)
                } else {
                    continue;
                };
                match Cbor::checked(&message)
                    .map_err(CodecError::custom)
                    .and_then(GossipMessage::read_cbor)
                {
                    Ok(GossipMessage::RootUpdate(root_update)) => {
                        swarm_observer.send((peer_id, GossipMessage::RootUpdate(root_update.clone_without_blocks())));
                        let _s = tracing::trace_span!("root update", root = %root_update.root);
                        let _s = _s.enter();
                        tracing::debug!(
                            "from {} with {} blocks, lamport: {}, offset: {:?}",
                            root_update.stream,
                            root_update.blocks.len(),
                            root_update.lamport,
                            root_update.offset
                        );
                        let mut lock = store.lock();
                        tracing::trace!("got store lock");
                        lock.received_lamport(root_update.lamport)
                            .expect("unable to update lamport");
                        drop(lock);
                        tracing::trace!("updated lamport");
                        if let Some(offset) = root_update.offset {
                            store.update_highest_seen(root_update.stream, offset);
                        }
                        let path = if root_update.blocks.is_empty() {
                            RootPath::SlowPath
                        } else {
                            RootPath::FastPath
                        };
                        for block in root_update.blocks {
                            let cid = *block.cid();
                            if let Err(err) = store.ipfs().insert(block) {
                                tracing::error!("{}", err);
                            } else {
                                tracing::trace!("{} written", display(cid));
                            }
                        }
                        match Link::try_from(root_update.root) {
                            Ok(root) => store.update_root(root_update.stream, root, RootSource::new(peer_id, path)),
                            Err(err) => tracing::error!("failed to parse link {}", err),
                        }
                    }
                    Ok(GossipMessage::RootMap(root_map)) => {
                        swarm_observer.send((peer_id, GossipMessage::RootMap(root_map.clone())));
                        let _s = tracing::trace_span!("root map", lamport = %root_map.lamport);
                        let _s = _s.enter();
                        tracing::debug!("with {} entries, lamport: {}", root_map.entries.len(), root_map.lamport);
                        store
                            .lock()
                            .received_lamport(root_map.lamport)
                            .expect("unable to update lamport");
                        for (idx, (stream, root)) in root_map.entries.into_iter().enumerate() {
                            if let Some((offset, _)) = root_map.offsets.get(idx) {
                                store.update_highest_seen(stream, *offset);
                            }
                            match Link::try_from(root) {
                                Ok(root) => {
                                    store.update_root(stream, root, RootSource::new(peer_id, RootPath::RootMap))
                                }
                                Err(err) => tracing::error!("failed to parse link {}", err),
                            }
                        }
                    }
                    Err(err) => tracing::debug!("received invalid gossip message; skipping. {}", err),
                }
            }
        })
    }
}

impl Drop for Gossip {
    fn drop(&mut self) {
        self.publish_handle.abort();
    }
}
