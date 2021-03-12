use crate::{BanyanStore, Link};
use actyxos_sdk::{NodeId, StreamId, StreamNr};
use anyhow::Result;
use ax_futures_util::stream::latest_channel;
use futures::prelude::*;
use ipfs_node::{Block, Cid, IpfsNode};
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "RootUpdateIo", try_from = "RootUpdateIo")]
struct RootUpdate {
    stream: StreamId,
    root: Cid,
    blocks: Vec<Block>,
}

#[derive(Serialize, Deserialize)]
struct RootUpdateIo {
    stream: String,
    root: String,
    blocks: Vec<(String, ByteBuf)>,
}

impl From<RootUpdate> for RootUpdateIo {
    fn from(value: RootUpdate) -> Self {
        Self {
            stream: value.stream.to_string(),
            root: value.root.to_string(),
            blocks: value
                .blocks
                .into_iter()
                .map(|block| (block.cid().to_string(), ByteBuf::from(block.data().to_vec())))
                .collect(),
        }
    }
}

impl TryFrom<RootUpdateIo> for RootUpdate {
    type Error = anyhow::Error;

    fn try_from(value: RootUpdateIo) -> Result<Self, Self::Error> {
        let root: Cid = value.root.parse()?;
        let stream: StreamId = StreamId::try_from(value.stream)?;
        let blocks = value
            .blocks
            .into_iter()
            .map(|(cid, data)| Ok(Block::new(cid.parse()?, data.to_vec())?))
            .collect::<Result<Vec<Block>>>()?;
        Ok(Self { root, stream, blocks })
    }
}

pub struct GossipV2 {
    tx: latest_channel::Sender<PublishUpdate>,
    publish_handle: tokio::task::JoinHandle<()>,
}

impl GossipV2 {
    pub fn new(ipfs: IpfsNode, node_id: NodeId, topic: String) -> Self {
        let (tx, mut rx) = latest_channel::channel::<PublishUpdate>();
        let publish_task = async move {
            while let Some(update) = rx.next().await {
                let root = Cid::from(update.root);
                let stream = node_id.stream(update.stream);
                let mut size = 0;
                let mut blocks = Vec::with_capacity(100);
                for link in update.links {
                    let cid = Cid::from(link);
                    if let Ok(Some(data)) = ipfs.lock_store().get_block(&cid) {
                        if size + data.len() > MAX_BROADCAST_BYTES {
                            break;
                        } else {
                            size += data.len();
                            blocks.push(Block::new_unchecked(cid, data));
                        }
                    }
                }
                let blob = serde_cbor::to_vec(&RootUpdate { root, stream, blocks }).unwrap();
                tracing::info!("broadcast_blob {}", blob.len());
                ipfs.broadcast(&topic, blob).ok();

                let blob = serde_cbor::to_vec(&RootUpdate {
                    root,
                    stream,
                    blocks: Default::default(),
                })
                .unwrap();
                tracing::info!("publish_blob {}", blob.len());
                let _ = ipfs.publish(&topic, blob);
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
                    match serde_cbor::from_slice::<RootUpdate>(&message) {
                        Ok(root_update) => {
                            tracing::debug!(
                                "received root update {} with {} blocks",
                                root_update.root,
                                root_update.blocks.len()
                            );
                            let mut bs = store.ipfs().lock_store();
                            let tmp = bs.temp_pin();
                            bs.assign_temp_pin(&tmp, std::iter::once(root_update.root)).ok();
                            bs.put_blocks(root_update.blocks, None).ok();
                            drop(bs);
                            if let Ok(root) = Link::try_from(root_update.root) {
                                store.update_root(root_update.stream, root);
                            }
                            drop(tmp);
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
