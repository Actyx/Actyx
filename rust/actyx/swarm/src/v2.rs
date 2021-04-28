use crate::{BanyanStore, Block, Ipfs, Link};
use actyxos_sdk::{NodeId, StreamId, StreamNr};
use anyhow::Result;
use ax_futures_util::stream::latest_channel;
use futures::prelude::*;
use libipld::{
    cbor::DagCborCodec,
    codec::{Codec, Decode, Encode},
    Cid, DagCbor,
};
use std::collections::BTreeSet;
use std::convert::TryFrom;

const MAX_BROADCAST_BYTES: usize = 1_000_000;

/// Update when we have rewritten a tree
#[derive(Debug)]
struct PublishUpdate {
    stream: StreamNr,
    root: Link,
    links: BTreeSet<Link>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RootUpdate {
    stream: StreamId,
    root: Cid,
    blocks: Vec<Block>,
}

impl Encode<DagCborCodec> for RootUpdate {
    fn encode<W: std::io::Write>(&self, c: DagCborCodec, w: &mut W) -> Result<()> {
        RootUpdateIo::from(self).encode(c, w)
    }
}

impl Decode<DagCborCodec> for RootUpdate {
    fn decode<R: std::io::Read + std::io::Seek>(c: DagCborCodec, r: &mut R) -> Result<Self> {
        let tmp = RootUpdateIo::decode::<R>(c, r)?;
        RootUpdate::try_from(tmp)
    }
}

#[derive(DagCbor)]
struct RootUpdateIo {
    stream: StreamId,
    root: Cid,
    blocks: Vec<(Cid, Vec<u8>)>,
}

impl From<&RootUpdate> for RootUpdateIo {
    fn from(value: &RootUpdate) -> Self {
        Self {
            stream: value.stream,
            root: value.root,
            blocks: value
                .blocks
                .iter()
                .map(|block| (*block.cid(), block.data().to_vec()))
                .collect(),
        }
    }
}

impl TryFrom<RootUpdateIo> for RootUpdate {
    type Error = anyhow::Error;

    fn try_from(value: RootUpdateIo) -> Result<Self, Self::Error> {
        let root: Cid = value.root;
        let stream = value.stream;
        let blocks = value
            .blocks
            .into_iter()
            .map(|(cid, data)| Block::new(cid, data.to_vec()))
            .collect::<Result<Vec<Block>>>()?;
        Ok(Self { root, stream, blocks })
    }
}

pub struct GossipV2 {
    tx: latest_channel::Sender<PublishUpdate>,
    publish_handle: tokio::task::JoinHandle<()>,
}

impl GossipV2 {
    pub fn new(ipfs: Ipfs, node_id: NodeId, topic: String) -> Self {
        let (tx, mut rx) = latest_channel::channel::<PublishUpdate>();
        let publish_task = async move {
            while let Some(update) = rx.next().await {
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
                let blob = DagCborCodec.encode(&RootUpdate { root, stream, blocks }).unwrap();
                tracing::debug!("broadcast_blob to pubsub {} {}", topic, blob.len());
                ipfs.broadcast(&topic, blob).ok();

                let blob = DagCborCodec
                    .encode(&RootUpdate {
                        root,
                        stream,
                        blocks: Default::default(),
                    })
                    .unwrap();
                tracing::trace!("publish_blob {}", blob.len());
                ipfs.publish(&topic, blob).ok();
            }
        };
        Self {
            tx,
            publish_handle: tokio::spawn(publish_task),
        }
    }

    pub fn publish(&self, stream: StreamNr, root: Link, links: BTreeSet<Link>) -> Result<()> {
        self.tx.send(PublishUpdate { stream, root, links })?;
        Ok(())
    }

    pub fn ingest(&self, store: BanyanStore, topic: String) -> Result<impl Future<Output = ()>> {
        let mut subscription = store.ipfs().subscribe(&topic)?;
        Ok(async move {
            loop {
                while let Some(message) = subscription.next().await {
                    match DagCborCodec.decode::<RootUpdate>(&message) {
                        Ok(root_update) => {
                            tracing::debug!(
                                "{} received root update {} with {} blocks",
                                store.ipfs().local_node_name(),
                                root_update.stream,
                                root_update.blocks.len()
                            );
                            match store.ipfs().create_temp_pin() {
                                Ok(tmp) => {
                                    if let Err(err) = store.ipfs().temp_pin(&tmp, &root_update.root) {
                                        tracing::error!("{}", err);
                                    }
                                    for block in &root_update.blocks {
                                        if let Err(err) = store.ipfs().insert(block) {
                                            tracing::error!("{}", err);
                                        }
                                    }
                                    match Link::try_from(root_update.root) {
                                        Ok(root) => store.update_root(root_update.stream, root),
                                        Err(err) => tracing::error!("failed to parse link {}", err),
                                    }
                                }
                                Err(err) => {
                                    tracing::error!("failed to create temp pin {}", err);
                                }
                            }
                        }
                        Err(err) => tracing::debug!("received invalid root update; skipping. {}", err),
                    }
                }
            }
        })
    }
}

impl Drop for GossipV2 {
    fn drop(&mut self) {
        self.publish_handle.abort();
    }
}
