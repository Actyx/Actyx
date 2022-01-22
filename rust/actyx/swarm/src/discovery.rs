//! Purpose of the discovery protocol.
//!
//! If a is connected to b and b is connected to c but a and c cannot discover each other via other
//! means, the discovery protocol will relay other addresses a and c can reach each other  accross b.
//! This could be done using gossip, but this means we need to flood the network regularly with this
//! information to reach new peers that joined the network. Instead we use banyan to disseminate this
//! information, because it is designed for this purpose.
//!
//! Other protocols that supplement the discovery protocol are identify, ping and mdns.
//!
//! - identify is a protocol used to tell you what address your peer observed for you. this is added
//! as an external address using [`NetworkBehaviourAction::ReportObservedAddress`]. Statically
//! configured external addresses are configured with [`Ipfs::add_external_address`].
//!
//! - ping is a protocol that keeps connections alive, and severs them when the peer is unresponsive.
//! Without ping every time you close all substreams the connection gets closed too. This prevents
//! unnecessary tcp connection churn.
//!
//! - mdns primary purpose is for discovering other nodes on the local network when no boostrap nodes
//! are necessary.
//!
//! Configuring nodes to maximize discoverability.
//!
//! Every network that has nodes which are part of the swarm should have at least one bootstrap node
//! configured. A bootstrap node is a node which has statically configured addresses to each other
//! bootstrap node in the swarm. Bootstrap addresses are configurable using [`ax_config`]. The purpose
//! of bootstrap nodes is to initially bootstrap the swarm and diagnose connectivity issues if they
//! occur. This makes mobile devices unsuitable as boostrap nodes because they may be physically moved
//! accross broadcast domains.
//!
//! A non bootstrap node needs to have at least one bootstrap node configured if mdns doesn't work
//! due to a firewall. If mdns does work it will discover it's local network bootstrap node
//! automatically.
//!
//! In some cases you will want to configure an `announceAddress` or `externalAddress`. The purpose
//! of the `announceAddress` is to ease the configuration of a network when dealing with multiple
//! NATs. When configuring a bootstrap node you are telling the node how to reach another peer,
//! while when configuring an external address you are telling other peers how to reach you, given
//! you have a bootstrap node in common.
use crate::{internal_app_id, BanyanStore};
use actyx_sdk::{tag, tags, Payload, StreamNr, Timestamp};
use anyhow::Result;
use fnv::{FnvHashMap, FnvHashSet};
use futures::stream::{Stream, StreamExt};
use ipfs_embed::multiaddr;
use libipld::cbor::DagCborCodec;
use libipld::codec::{Codec, Decode, Encode};
use libipld::DagCbor;
use std::future::Future;
use std::io::{Read, Seek, Write};
use std::time::Duration;
use tokio::time::timeout;
use trees::query::{LamportQuery, TagExprQuery, TimeQuery};
use trees::tags::{ScopedTag, ScopedTagSet, TagScope};
use trees::AxKey;

#[derive(DagCbor, Debug)]
#[allow(clippy::enum_variant_names)]
enum Event {
    /// Listening for incoming connections on a new address.
    NewListenAddr(PeerId, Multiaddr),
    /// Not listening for incoming connections on a previous address.
    ExpiredListenAddr(PeerId, Multiaddr),
    /// An external address was added with `Swarm::add_external_address`.
    NewExternalAddr(PeerId, Multiaddr),
    /// An external address was removed with `Swarm::remove_external_address`.
    ExpiredExternalAddr(PeerId, Multiaddr),
    /// A peer reported they observed an external address.
    NewObservedAddr(PeerId, Multiaddr),
    /// Address dropped due to peers reporting different addresses. A maximum of
    /// eight addresses are kept. This is implemented in libp2p-swarm.
    ExpiredObservedAddr(PeerId, Multiaddr),
}

impl Event {
    fn peer_id(&self) -> &ipfs_embed::PeerId {
        match self {
            Self::NewListenAddr(peer, _) => &peer.0,
            Self::ExpiredListenAddr(peer, _) => &peer.0,
            Self::NewExternalAddr(peer, _) => &peer.0,
            Self::ExpiredExternalAddr(peer, _) => &peer.0,
            Self::NewObservedAddr(peer, _) => &peer.0,
            Self::ExpiredObservedAddr(peer, _) => &peer.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

fn decode_event(e: Result<(u64, AxKey, Payload)>, my_peer_id: ipfs_embed::PeerId) -> Option<Event> {
    let (_off, _key, event) = match e {
        Ok(event) => event,
        Err(err) => {
            tracing::warn!("store error: {}", err);
            return None;
        }
    };
    let event: Event = match DagCborCodec.decode(event.as_slice()) {
        Ok(event) => event,
        Err(err) => {
            tracing::debug!("decoding error: {}", err);
            return None;
        }
    };
    if *event.peer_id() == my_peer_id {
        None
    } else {
        Some(event)
    }
}

pub async fn discovery_ingest(store: BanyanStore) {
    let mut tags: ScopedTagSet = tags!("discovery").into();
    tags.insert(ScopedTag::new(TagScope::Internal, tag!("app_id:com.actyx")));
    let query = TagExprQuery::new(
        vec![tags],
        LamportQuery::all(),
        TimeQuery::from(Timestamp::now() - 1_000_000_000_000..),
    );
    let mut stream = store.stream_filtered_stream_ordered(query);
    let peer_id = store.ipfs().local_peer_id();

    // first catch up and build a list, we won’t want to spam the address book
    let mut addresses = FnvHashMap::<PeerId, FnvHashSet<Multiaddr>>::default();
    while let Ok(Some(event)) = timeout(Duration::from_secs(3), stream.next()).await {
        let event = match decode_event(event, peer_id) {
            Some(e) => e,
            None => continue,
        };
        tracing::debug!("discovery_ingest (catch-up) {:?}", event);
        match event {
            Event::NewListenAddr(peer, addr)
            | Event::NewExternalAddr(peer, addr)
            | Event::NewObservedAddr(peer, addr) => {
                addresses.entry(peer).or_default().insert(addr);
            }
            Event::ExpiredListenAddr(peer, addr)
            | Event::ExpiredExternalAddr(peer, addr)
            | Event::ExpiredObservedAddr(peer, addr) => {
                addresses.entry(peer).or_default().remove(&addr);
            }
        }
    }
    for (peer, addrs) in addresses {
        for addr in addrs {
            store.ipfs().add_address(&peer.into(), addr.into());
        }
    }

    // then switch to live mode
    tracing::debug!("discovery_ingest switching to live mode");
    while let Some(event) = stream.next().await {
        let event = match decode_event(event, peer_id) {
            Some(e) => e,
            None => continue,
        };
        tracing::debug!("discovery_ingest {:?}", event);
        match event {
            Event::NewListenAddr(peer, addr)
            | Event::NewExternalAddr(peer, addr)
            | Event::NewObservedAddr(peer, addr) => store.ipfs().add_address(&peer.into(), addr.into()),
            Event::ExpiredListenAddr(peer, addr)
            | Event::ExpiredExternalAddr(peer, addr)
            | Event::ExpiredObservedAddr(peer, addr) => store.ipfs().remove_address(&peer.into(), &addr.into()),
        }
    }
}

struct Dialer {
    backoff: Duration,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl Dialer {
    fn new(backoff: Duration, task: tokio::task::JoinHandle<()>) -> Self {
        Self {
            backoff,
            task: Some(task),
        }
    }
}

impl Drop for Dialer {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

fn is_loopback(addr: &ipfs_embed::Multiaddr) -> bool {
    match addr.iter().next() {
        Some(multiaddr::Protocol::Ip4(a)) => a.is_loopback(),
        Some(multiaddr::Protocol::Ip6(a)) => a.is_loopback(),
        _ => false,
    }
}

pub fn discovery_publish(
    store: BanyanStore,
    mut stream: impl Stream<Item = ipfs_embed::Event> + Unpin,
    nr: StreamNr,
    external: FnvHashSet<ipfs_embed::Multiaddr>,
    enable_discovery: bool,
    to_warn: Vec<ipfs_embed::PeerId>,
) -> Result<impl Future<Output = ()>> {
    let mut buffer = vec![];
    let tags = tags!("discovery");
    let peer_id: PeerId = store.ipfs().local_peer_id().into();
    let mut dialers = FnvHashMap::<_, Dialer>::default();
    let mut to_warn = to_warn
        .into_iter()
        .map(|id| (id, true))
        .collect::<FnvHashMap<_, bool>>();
    Ok(async move {
        while let Some(event) = stream.next().await {
            tracing::trace!("discovery_publish {:?}", event);
            let event = match event {
                ipfs_embed::Event::NewListenAddr(_, addr) => {
                    if !is_loopback(&addr) {
                        Event::NewListenAddr(peer_id, addr.into())
                    } else {
                        continue;
                    }
                }
                ipfs_embed::Event::ExpiredListenAddr(_, addr) => {
                    if !is_loopback(&addr) {
                        Event::ExpiredListenAddr(peer_id, addr.into())
                    } else {
                        continue;
                    }
                }
                ipfs_embed::Event::NewExternalAddr(addr) => {
                    if external.contains(&addr) {
                        Event::NewExternalAddr(peer_id, addr.into())
                    } else {
                        Event::NewObservedAddr(peer_id, addr.into())
                    }
                }
                ipfs_embed::Event::ExpiredExternalAddr(addr) => {
                    if external.contains(&addr) {
                        Event::ExpiredExternalAddr(peer_id, addr.into())
                    } else {
                        Event::ExpiredObservedAddr(peer_id, addr.into())
                    }
                }
                ipfs_embed::Event::Discovered(peer) => {
                    store.ipfs().dial(&peer);
                    continue;
                }
                ipfs_embed::Event::Unreachable(peer) => {
                    if let Some(warn) = to_warn.get_mut(&peer) {
                        if *warn {
                            tracing::warn!(id = display(&peer), "connection failed to initial peer");
                        } else {
                            tracing::debug!(id = display(&peer), "connection failed to initial peer");
                        }
                        *warn = false;
                    } else {
                        tracing::debug!(id = display(&peer), "connection failed");
                    }
                    let ipfs = store.ipfs().clone();
                    let backoff = if let Some(dialer) = dialers.remove(&peer) {
                        dialer.backoff.saturating_mul(2).min(Duration::from_secs(60))
                    } else {
                        Duration::from_secs(1)
                    };
                    let task = tokio::spawn(async move {
                        tokio::time::sleep(backoff).await;
                        ipfs.dial(&peer);
                    });
                    dialers.insert(peer, Dialer::new(backoff, task));
                    continue;
                }
                ipfs_embed::Event::Connected(peer) => {
                    if let Some(warn) = to_warn.get_mut(&peer) {
                        tracing::info!(id = display(&peer), "connected to initial peer");
                        *warn = false;
                    } else {
                        tracing::debug!(id = display(&peer), "connected");
                    }
                    // dropping the Dialer will kill the task
                    dialers.remove(&peer);
                    continue;
                }
                ipfs_embed::Event::Disconnected(peer) => {
                    if let Some(warn) = to_warn.get_mut(&peer) {
                        tracing::info!(id = display(&peer), "disconnected from initial peer");
                        *warn = false;
                    } else {
                        tracing::debug!(id = display(&peer), "disconnected");
                    }
                    // dialing on disconnected ensures the unreachable event fires.
                    store.ipfs().dial(&peer);
                    continue;
                }
                ipfs_embed::Event::NewInfo(peer) => {
                    if let Some(info) = store.ipfs().peer_info(&peer) {
                        if let Some(rtt) = info.full_rtt() {
                            if rtt.failures() > 0 {
                                tracing::info!(peer = display(peer), info = debug(rtt), "ping failure");
                            } else if rtt.current().as_secs() >= 1 {
                                let addrs = store
                                    .ipfs()
                                    .connections()
                                    .into_iter()
                                    .filter(|x| x.0 == peer)
                                    .map(|x| x.1)
                                    .collect::<Vec<_>>();
                                tracing::warn!(
                                    peer = display(peer),
                                    addr = debug(&addrs),
                                    info = debug(rtt),
                                    "slow ping time"
                                );
                            }
                        }
                    }
                    continue;
                }
                _ => continue,
            };
            if enable_discovery {
                buffer.clear();
                if let Err(err) = event.encode(DagCborCodec, &mut buffer) {
                    tracing::warn!("{}", err);
                    continue;
                }
                if let Err(err) = store
                    .append(
                        nr,
                        internal_app_id(),
                        vec![(tags.clone(), Payload::from_slice(&buffer))],
                    )
                    .await
                {
                    tracing::warn!("error appending discovery: {}", err);
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_discovery() -> Result<()> {
        util::setup_logger();
        let a = BanyanStore::test("a").await?;
        let b = BanyanStore::test("b").await?;
        let c = BanyanStore::test("c").await?;
        let a_id = a.ipfs().local_peer_id();
        let b_id = b.ipfs().local_peer_id();
        let c_id = c.ipfs().local_peer_id();
        a.ipfs()
            .listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?
            .next()
            .await
            .unwrap();
        b.ipfs()
            .listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?
            .next()
            .await
            .unwrap();
        c.ipfs()
            .listen_on("/ip4/127.0.0.1/tcp/0".parse()?)?
            .next()
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        a.ipfs().add_address(&b_id, b.ipfs().listeners()[0].clone());
        c.ipfs().add_address(&b_id, b.ipfs().listeners()[0].clone());
        loop {
            if a.ipfs().is_connected(&c_id) && c.ipfs().is_connected(&a_id) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await
        }
        Ok(())
    }
}
