use crate::BanyanStore;
use actyxos_sdk::{tags, Payload, StreamNr};
use anyhow::Result;
use fnv::FnvHashSet;
use futures::stream::StreamExt;
use libipld::cbor::DagCborCodec;
use libipld::codec::{Codec, Decode, Encode};
use libipld::DagCbor;
use std::future::Future;
use std::io::{Read, Seek, Write};
use trees::axtrees::TagsQuery;

#[derive(DagCbor, Debug)]
#[allow(clippy::enum_variant_names)]
enum Event {
    NewListenAddr(PeerId, Multiaddr),
    ExpiredListenAddr(PeerId, Multiaddr),
    NewExternalAddr(PeerId, Multiaddr),
    NewObservedAddr(PeerId, Multiaddr),
}

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
struct PeerId(ipfs_embed::PeerId);

impl From<ipfs_embed::PeerId> for PeerId {
    fn from(peer: ipfs_embed::PeerId) -> Self {
        Self(peer)
    }
}

impl From<PeerId> for ipfs_embed::PeerId {
    fn from(peer: PeerId) -> Self {
        peer.0
    }
}

impl<C: Codec> Encode<C> for PeerId
where
    String: Encode<C>,
{
    fn encode<W: Write>(&self, c: C, w: &mut W) -> Result<()> {
        self.0.to_string().encode(c, w)
    }
}

impl<C: Codec> Decode<C> for PeerId
where
    String: Decode<C>,
{
    fn decode<R: Read + Seek>(c: C, r: &mut R) -> Result<Self> {
        Ok(Self(String::decode(c, r)?.parse()?))
    }
}

#[derive(Clone, Debug)]
#[repr(transparent)]
struct Multiaddr(ipfs_embed::Multiaddr);

impl From<ipfs_embed::Multiaddr> for Multiaddr {
    fn from(addr: ipfs_embed::Multiaddr) -> Self {
        Self(addr)
    }
}

impl From<Multiaddr> for ipfs_embed::Multiaddr {
    fn from(addr: Multiaddr) -> Self {
        addr.0
    }
}

impl<C: Codec> Encode<C> for Multiaddr
where
    String: Encode<C>,
{
    fn encode<W: Write>(&self, c: C, w: &mut W) -> Result<()> {
        self.0.to_string().encode(c, w)
    }
}

impl<C: Codec> Decode<C> for Multiaddr
where
    String: Decode<C>,
{
    fn decode<R: Read + Seek>(c: C, r: &mut R) -> Result<Self> {
        Ok(Self(String::decode(c, r)?.parse()?))
    }
}

pub async fn discovery_ingest(store: BanyanStore) {
    let tags = tags!("discovery");
    let query = TagsQuery::new(vec![tags]);
    let mut stream = store.stream_filtered_stream_ordered(query);
    while let Some(event) = stream.next().await {
        let event = match event {
            Ok(event) => event,
            Err(err) => {
                tracing::warn!("{}", err);
                continue;
            }
        };
        let event: Event = match DagCborCodec.decode(event.2.as_slice()) {
            Ok(event) => event,
            Err(err) => {
                tracing::warn!("{}", err);
                continue;
            }
        };
        match event {
            Event::NewListenAddr(peer, addr) => store.ipfs().add_address(&peer.into(), addr.into()),
            Event::ExpiredListenAddr(peer, addr) => store.ipfs().remove_address(&peer.into(), &addr.into()),
            Event::NewExternalAddr(peer, addr) => store.ipfs().add_address(&peer.into(), addr.into()),
            Event::NewObservedAddr(peer, addr) => store.ipfs().add_address(&peer.into(), addr.into()),
        }
    }
}

pub fn discovery_publish(
    store: BanyanStore,
    nr: StreamNr,
    external: FnvHashSet<ipfs_embed::Multiaddr>,
) -> Result<impl Future<Output = ()>> {
    let mut stream = store.ipfs().event_stream();
    let mut buffer = vec![];
    let tags = tags!("discovery");
    let peer_id: PeerId = store.ipfs().local_peer_id().into();
    Ok(async move {
        while let Some(event) = stream.next().await {
            let event = match event {
                ipfs_embed::Event::NewListenAddr(addr) => Event::NewListenAddr(peer_id, addr.into()),
                ipfs_embed::Event::ExpiredListenAddr(addr) => Event::ExpiredListenAddr(peer_id, addr.into()),
                ipfs_embed::Event::NewExternalAddr(addr) => {
                    if external.contains(&addr) {
                        Event::NewExternalAddr(peer_id, addr.into())
                    } else {
                        Event::NewObservedAddr(peer_id, addr.into())
                    }
                }
                ipfs_embed::Event::Discovered(peer) => {
                    if let Err(err) = store.ipfs().dial(&peer) {
                        tracing::warn!("failed to dial peer {}", err);
                    }
                    continue;
                }
            };
            buffer.clear();
            if let Err(err) = event.encode(DagCborCodec, &mut buffer) {
                tracing::warn!("{}", err);
                continue;
            }
            if let Err(err) = store
                .append(nr, vec![(tags.clone(), Payload::from_slice(&buffer))])
                .await
            {
                tracing::warn!("error appending discovery: {}", err);
            }
        }
    })
}
