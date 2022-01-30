mod behaviour;
mod handler;
mod protocol;
#[cfg(test)]
mod tests;

pub use behaviour::StreamingResponse;
pub use protocol::ProtocolError;

use self::handler::{ResponseFuture, Spawner};
use crate::Codec;
use futures::channel::mpsc;
use libp2p::{core::connection::ConnectionId, PeerId};
use std::{fmt::Debug, sync::Arc, time::Duration};

pub enum Event<T: Codec> {
    RequestReceived {
        peer_id: PeerId,
        connection: ConnectionId,
        request: T::Request,
        channel: mpsc::Sender<T::Response>,
    },
}

impl<T: Codec> Debug for Event<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestReceived {
                peer_id,
                connection,
                request,
                ..
            } => f
                .debug_struct("RequestReceived")
                .field("peer_id", peer_id)
                .field("connection", connection)
                .field("request", request)
                .finish(),
        }
    }
}

pub struct StreamingResponseConfig {
    spawner: Spawner,
    request_timeout: Duration,
    max_message_size: u32,
    response_send_buffer_size: usize,
    keep_alive: bool,
}

impl StreamingResponseConfig {
    /// Spawn response stream handling tasks using the given function
    ///
    /// This function may be called from an arbitrary context, you cannot assume that because
    /// you’re using Tokio this will happen on a Tokio thread. Hence it is necessary to point
    /// to the target thread pool directly, e.g. by using a runtime handle.
    ///
    /// If this method is not used, tasks will be polled via the Swarm, which may be an I/O
    /// bottleneck.
    pub fn with_spawner(self, spawner: impl Fn(ResponseFuture) -> ResponseFuture + Send + Sync + 'static) -> Self {
        Self {
            spawner: Arc::new(spawner),
            ..self
        }
    }
    /// Timeout for the transmission of the request to the peer, default is 10sec
    pub fn with_request_timeout(self, request_timeout: Duration) -> Self {
        Self {
            request_timeout,
            ..self
        }
    }
    /// Maximum message size permitted for requests and responses
    ///
    /// The maximum is 4GiB, the default 1MB. Sending huge messages requires corresponding
    /// buffers and may not be desirable.
    pub fn with_max_message_size(self, max_message_size: u32) -> Self {
        Self {
            max_message_size,
            ..self
        }
    }
    /// Set the queue size in messages for the channel created for incoming requests
    ///
    /// All channels are bounded in size and use back-pressure. This channel size allows some
    /// decoupling between response generation and network transmission. Default is 128.
    pub fn with_response_send_buffer_size(self, response_send_buffer_size: usize) -> Self {
        Self {
            response_send_buffer_size,
            ..self
        }
    }
    /// If this is set to true, then this behaviour will keep the connection alive
    ///
    /// Otherwise the connection is released (i.e. closed if no other behaviour keeps it alive)
    /// when there are no active requests ongoing. Default is `false`.
    pub fn with_keep_alive(self, keep_alive: bool) -> Self {
        Self { keep_alive, ..self }
    }
}

impl Default for StreamingResponseConfig {
    fn default() -> Self {
        Self {
            spawner: Arc::new(|f| f),
            request_timeout: Duration::from_secs(10),
            max_message_size: 1_000_000,
            response_send_buffer_size: 128,
            keep_alive: false,
        }
    }
}
