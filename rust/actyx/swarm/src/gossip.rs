use crate::{BanyanStore, Block, Ipfs, Link};
use actyxos_sdk::{LamportTimestamp, NodeId, StreamId, StreamNr, Timestamp};
use anyhow::Result;
use ax_futures_util::stream::latest_channel;
use futures::prelude::*;
use libipld::{
    cbor::DagCborCodec,
    codec::{Codec, Decode, Encode},
    Cid, DagCbor,
};
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RootUpdate {
    stream: StreamId,
    root: Cid,
    blocks: Vec<Block>,
    lamport: LamportTimestamp,
    time: Timestamp,
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
#[ipld(repr = "tuple")]
struct RootUpdateIo {
    stream: StreamId,
    root: Cid,
    blocks: Vec<(Cid, Vec<u8>)>,
    lamport: LamportTimestamp,
    time: Timestamp,
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
            lamport: value.lamport,
            time: value.time,
        }
    }
}

impl TryFrom<RootUpdateIo> for RootUpdate {
    type Error = anyhow::Error;

    fn try_from(value: RootUpdateIo) -> Result<Self, Self::Error> {
        let blocks = value
            .blocks
            .into_iter()
            .map(|(cid, data)| Block::new(cid, data.to_vec()))
            .collect::<Result<Vec<Block>>>()?;
        Ok(Self {
            root: value.root,
            stream: value.stream,
            lamport: value.lamport,
            time: value.time,
            blocks,
        })
    }
}

#[derive(Debug, Eq, PartialEq, DagCbor)]
#[ipld(repr = "int-tuple")]
enum GossipMessage {
    #[ipld(repr = "value")]
    RootUpdate(RootUpdate),
    #[ipld(repr = "value")]
    RootMap(RootMap),
}

#[derive(Debug, Eq, PartialEq, DagCbor, Default)]
#[ipld(repr = "tuple")]
pub struct RootMap {
    entries: BTreeMap<StreamId, Cid>,
    lamport: LamportTimestamp,
    time: Timestamp,
}

pub struct Gossip {
    tx: latest_channel::Sender<PublishUpdate>,
    publish_handle: tokio::task::JoinHandle<()>,
}

impl Gossip {
    pub fn new(ipfs: Ipfs, node_id: NodeId, topic: String, enable_fast_path: bool, enable_slow_path: bool) -> Self {
        let (tx, mut rx) = latest_channel::channel::<PublishUpdate>();
        let publish_task = async move {
            while let Some(update) = rx.next().await {
                let time = Timestamp::now();
                let lamport = update.lamport;
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

                if enable_fast_path {
                    let root_update = RootUpdate {
                        root,
                        stream,
                        blocks,
                        lamport,
                        time,
                    };
                    let blob = DagCborCodec.encode(&GossipMessage::RootUpdate(root_update)).unwrap();
                    tracing::trace!("broadcast_blob {} {}", stream, blob.len());
                    if let Err(err) = ipfs.broadcast(&topic, blob) {
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
                    };
                    let blob = DagCborCodec.encode(&GossipMessage::RootUpdate(root_update)).unwrap();
                    tracing::trace!("publish_blob {} {}", stream, blob.len());
                    if let Err(err) = ipfs.publish(&topic, blob) {
                        tracing::error!("publish failed: {}", err);
                    }
                }
            }
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
    ) -> Result<()> {
        self.tx.send(PublishUpdate {
            stream,
            root,
            links,
            lamport,
        })?;
        Ok(())
    }

    pub fn publish_root_map(&self, store: BanyanStore, topic: String, interval: Duration) -> impl Future<Output = ()> {
        async move {
            loop {
                tokio::time::sleep(interval).await;
                let guard = store.lock();
                let entries = guard.root_map();
                let lamport = guard.data.lamport.get();
                drop(guard);
                let time = Timestamp::now();
                let msg = GossipMessage::RootMap(RootMap { entries, lamport, time });
                let blob = DagCborCodec.encode(&msg).unwrap();
                if let Err(err) = store.ipfs().publish(&topic, blob) {
                    tracing::error!("publish root map failed: {}", err);
                }
            }
        }
    }

    pub fn ingest(&self, store: BanyanStore, topic: String) -> Result<impl Future<Output = ()>> {
        let mut subscription = store.ipfs().subscribe(&topic)?;
        Ok(async move {
            loop {
                while let Some(message) = subscription.next().await {
                    match DagCborCodec.decode::<GossipMessage>(&message) {
                        Ok(GossipMessage::RootUpdate(root_update)) => {
                            tracing::debug!(
                                "{} received root update {} with {} blocks",
                                store.ipfs().local_node_name(),
                                root_update.stream,
                                root_update.blocks.len()
                            );
                            store
                                .lock()
                                .received_lamport(root_update.lamport)
                                .expect("unable to update lamport");
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
                        Ok(GossipMessage::RootMap(root_map)) => {
                            tracing::debug!("{} received root map", store.ipfs().local_node_name());
                            store
                                .lock()
                                .received_lamport(root_map.lamport)
                                .expect("unable to update lamport");
                            for (stream, root) in root_map.entries {
                                match Link::try_from(root) {
                                    Ok(root) => store.update_root(stream, root),
                                    Err(err) => tracing::error!("failed to parse link {}", err),
                                }
                            }
                        }
                        Err(err) => tracing::debug!("received invalid gossip message; skipping. {}", err),
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use libipld::multihash::{Code, MultihashDigest};

    #[test]
    fn test_decode_root_update() {
        #[rustfmt::skip]
        let cbor = [
            0x82, // array(2)
                0x00, // unsigned(0)
                0x85, // array(5)
                    0x82, // array(2)
                        0x58, 0x20, // bytes(32)
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                            0xff, 0xff, 0xff, 0xff,
                        0x18, 0x2a, // unsigned(42)
                    0xd8, 0x2a, // tag(42)
                        0x58, 0x25, // bytes(37)
                            0x00, 0x01, 0x00, 0x12,
                            0x20, 0xE3, 0xB0, 0xC4,
                            0x42, 0x98, 0xFC, 0x1C,
                            0x14, 0x9A, 0xFB, 0xF4,
                            0xC8, 0x99, 0x6F, 0xB9,
                            0x24, 0x27, 0xAE, 0x41,
                            0xE4, 0x64, 0x9B, 0x93,
                            0x4C, 0xA4, 0x95, 0x99,
                            0x1B, 0x78, 0x52, 0xB8, 0x55,
                    0x80, // array(0)
                    0x00, // unsigned(0)
                    0x00, // unsigned(0)
        ];
        let root_update = GossipMessage::RootUpdate(RootUpdate {
            stream: NodeId::from_bytes(&[0xff; 32]).unwrap().stream(42.into()),
            root: Cid::new_v1(0x00, Code::Sha2_256.digest(&[])),
            blocks: Default::default(),
            lamport: Default::default(),
            time: Default::default(),
        });
        let msg = DagCborCodec.encode(&root_update).unwrap();
        assert_eq!(msg, cbor);
        let root_update2 = DagCborCodec.decode(&msg).unwrap();
        assert_eq!(root_update, root_update2);
    }

    #[test]
    fn test_decode_root_map() {
        #[rustfmt::skip]
        let cbor = [
            0x82, // array(3)
                0x01, // unsigned(1)
                0x83, // array(2)
                    0xa0, // map(0)
                    0x00, // unsigned(0)
                    0x00, // unsigned(0)
        ];
        let root_map = GossipMessage::RootMap(Default::default());
        let msg = DagCborCodec.encode(&root_map).unwrap();
        assert_eq!(msg, cbor);
        let root_map2 = DagCborCodec.decode(&msg).unwrap();
        assert_eq!(root_map, root_map2);
    }
}
