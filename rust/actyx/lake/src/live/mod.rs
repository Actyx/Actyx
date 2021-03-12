use anyhow::Result;
use futures::future;
use futures::stream::{Stream, StreamExt};
use ipfs_node::IpfsNode;
use serde::de::DeserializeOwned;
use tracing::*;
use util::serde_util::from_json_or_cbor_slice;

#[derive(Clone)]
pub struct LiveEvents {
    client: IpfsNode,
    retry_on_error: bool, // should the stream retry indefinitely on errors?
}

#[derive(Debug, Clone)]
pub struct Topic(pub String);

impl LiveEvents {
    pub fn new(client: &IpfsNode) -> Self {
        LiveEvents {
            client: client.clone(),
            retry_on_error: true,
        }
    }

    pub fn without_retry(client: &IpfsNode) -> Self {
        LiveEvents {
            client: client.clone(),
            retry_on_error: false,
        }
    }

    // neverending stream, all errors are signalled, but the stream is attempted indefinitely
    pub fn listen_on<T>(&self, topic: &Topic) -> Result<impl Stream<Item = T>>
    where
        T: DeserializeOwned + Send + Clone,
    {
        Ok(self.listen_raw(&topic)?.filter_map(|raw_pubsub_ev| {
            let res = match from_json_or_cbor_slice::<T>(raw_pubsub_ev.as_slice()) {
                Ok(x) => Some(x),
                Err(err) => {
                    warn!("Could not parse line as Actyx PubSub Message: {}", err);
                    None
                }
            };
            future::ready(res)
        }))
    }

    pub fn listen_raw(&self, topic: &Topic) -> Result<impl Stream<Item = Vec<u8>>> {
        self.client.subscribe(&topic.0)
    }
}
