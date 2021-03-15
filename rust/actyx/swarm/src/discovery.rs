use crate::Ipfs;
use anyhow::Result;
use futures::stream::StreamExt;
use libipld::cbor::DagCborCodec;
use libipld::codec::Codec;
use libipld::DagCbor;
use libp2p::{Multiaddr, PeerId};
use std::future::Future;
use std::time::Duration;

const DISCOVERY_TOPIC: &str = "discovery";
const DISCOVERY_INTERVAL: Duration = Duration::from_secs(30);

#[derive(DagCbor, Debug)]
enum DiscoveryMessage {
    Connections(Vec<(String, String)>),
}

impl From<Vec<(PeerId, Multiaddr)>> for DiscoveryMessage {
    fn from(conns: Vec<(PeerId, Multiaddr)>) -> Self {
        Self::Connections(conns.into_iter().map(|(p, a)| (p.to_string(), a.to_string())).collect())
    }
}

pub async fn discovery_publish(ipfs: Ipfs) {
    loop {
        tokio::time::sleep(DISCOVERY_INTERVAL).await;
        let msg = DiscoveryMessage::from(ipfs.connections());
        let bytes = DagCborCodec.encode(&msg).unwrap();
        ipfs.publish(DISCOVERY_TOPIC, bytes).ok();
    }
}

pub fn discovery_ingest(ipfs: Ipfs) -> Result<impl Future<Output = ()>> {
    let mut subscription = ipfs.subscribe(DISCOVERY_TOPIC)?;
    Ok(async move {
        while let Some(msg) = subscription.next().await {
            if let Ok(DiscoveryMessage::Connections(conns)) = DagCborCodec.decode(&msg) {
                for (peer_id, addr) in conns {
                    if let (Ok(peer_id), Ok(addr)) = (peer_id.parse(), addr.parse()) {
                        if let Err(err) = ipfs.dial_address(&peer_id, addr) {
                            tracing::error!("failed to dial peer: {}", err);
                        }
                    }
                }
            }
        }
    })
}
