//! Generic streaming request/response protocol.
//!
//! ## General Usage
//!
//! [`StreamingResponse`] is a `NetworkBehaviour` that implements a generic
//! request/response protocol or protocol family, whereby each request is sent
//! over a new substream on a connection. `StreamingResponse` is generic over the
//! actual messages being sent, which are defined in terms of a [`Codec`].
//! Creating a request/response protocol thus amounts to providing an
//! implementation of this trait which can then be given to
//! [`StreamingResponseConfig::new`], and finally to [`StreamingResponse::new`].
//! Further configuration options are available said config. For convenience, a
//! default implementation for [`StreamingResponse`] is provided.
//!
//! Requests are sent using [`StreamingResponse::request`] and the
//! responses received as [`StreamingResponseEvent::ResponseReceived`].
//!
//! Individual responses are sent using [`StreamingResponse::respond`]
//! upon receiving a [`StreamingResponseEvent::Request`]. The response stream can
//! be finalized by calling [`StreamingResponse::finish_response`],
//! which will result in the emission of an
//! [`StreamingResponseEvent::ResponseFinished`] on the requester's side. After
//! that, the response channel can't be used anymore.
//!
//! An ongoing request is cancelled if either the peer disconnects, or a
//! [`StreamingResponseMessage::CancelRequest`] message is sent.
//!
//! ## Protocol Families
//!
//! A single [`StreamingResponse`] instance can be used with an entire
//! protocol family that share the same request and response types. For that
//! purpose, [`Codec`] is typically instantiated with a sum type.
//!
//! ## Differences to `libp2p::request_response`
//!
//! The ergonomics of this behaviour are inspired by the
//! `libp2p::request_response` implementation. However, it enables the exchange
//! of multiple response frames per request. Currently, it does neither support
//! timeouts nor signalling of successful commits of outbound messages to the
//! underlying transport mechanism. Sending requests and/or responses is a
//! fire-and-forget action. Only if the remote peer is disconnected, consumer
//! code will be notified through [`CancellationReason::PeerDisconnected`] via
//! [`StreamingResponseEvent::ResponseFinished`].
//! Another notable difference is that this behaviour won't initiate any dialing
//! attempts, thus this behaviour needs to be wrapped inside another behaviour
//! providing dialing functionality.
//!
//! ## Parallelism and response frame ordering
//!
//! Internally, this behaviour uses `libp2p::swarm::OneShotHandler`, where for
//! each request a new substream is created. Meaning, that for each request and
//! all subsequent responses, this substream is used. Given the asynchronous
//! nature of the internals of `libp2p`, smaller response frames might make it to
//! the recipient earlier than bigger ones. Each response frame includes a
//! monotonic sequence number, which can be used for ordering purposes. However,
//! users can also set [`StreamingResponseConfig::ordered_outgoing`] flag, which
//! will commit individual responses sequentially to the underlying transport
//! mechanism.

use crate::handler::IntoHandler;
use derive_more::{Add, Deref, Display, Sub};
use futures::channel::mpsc;
use handler::Request;
use libp2p::{
    core::connection::ConnectionId,
    swarm::{NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, PollParameters},
    PeerId,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::VecDeque,
    fmt::Debug,
    marker::PhantomData,
    task::{Context, Poll},
    time::Duration,
};

mod handler;
mod protocol;
mod protocol_v2;
mod upgrade;

#[cfg(test)]
mod tests;

pub use handler::Response;
pub use protocol_v2::ProtocolError;

/// A [`Codec`] defines the request and response types for a [`StreamingResponse`]
/// protocol. Request and responses are encoded / decoded using `serde_cbor`, so
/// `Serialize` and `Deserialize` impls have to be provided. Implement this trait
/// to specialize the [`StreamingResponse`].
pub trait Codec {
    type Request: Send + Serialize + DeserializeOwned + std::fmt::Debug + 'static;
    type Response: Send + Serialize + DeserializeOwned + std::fmt::Debug + 'static;

    /// The first protocol name is used for the v2 protocol, the second for v1.
    fn protocol_info() -> [&'static str; 2];
    fn info_v1() -> &'static str {
        Self::protocol_info()[1]
    }
    fn info_v2() -> &'static str {
        Self::protocol_info()[0]
    }
}

#[derive(
    Debug, Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Display, Add, Sub, Deref,
)]
// SequenceNo for responses
pub struct SequenceNo(pub(crate) u64);
impl SequenceNo {
    pub fn increment(&mut self) {
        self.0 += 1
    }
}

pub struct RequestReceived<T: Codec> {
    pub peer_id: PeerId,
    pub connection: ConnectionId,
    pub request: T::Request,
    pub channel: mpsc::Sender<T::Response>,
}

impl<T: Codec> Debug for RequestReceived<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestReceived")
            .field("peer_id", &self.peer_id)
            .field("connection", &self.connection)
            .field("request", &self.request)
            .finish()
    }
}

pub struct StreamingResponseConfig {
    request_timeout: Duration,
    max_message_size: u32,
    response_send_buffer_size: usize,
    keep_alive: bool,
}

impl StreamingResponseConfig {
    /// Timeout for the transmission of the request to the peer, default is 10sec
    pub fn with_request_timeout(self, request_timeout: Duration) -> Self {
        Self {
            request_timeout,
            ..self
        }
    }
    /// Maximum message size permitted for requests and responses (limited to 0xfeffffff !)
    ///
    /// The maximum is slightly below 4GiB, the default 1MB. Sending huge messages requires corresponding
    /// buffers and may not be desirable.
    pub fn with_max_message_size(self, max_message_size: u32) -> Self {
        if max_message_size >= 0xff000000 {
            panic!(
                "max_message_size {} is beyond the limit of {}",
                max_message_size, 0xfeffffffu32
            );
        }
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
            request_timeout: Duration::from_secs(10),
            max_message_size: 1_000_000,
            response_send_buffer_size: 128,
            keep_alive: false,
        }
    }
}

pub struct StreamingResponse<T: Codec + Send + 'static> {
    config: StreamingResponseConfig,
    events: VecDeque<RequestReceived<T>>,
    requests: VecDeque<NetworkBehaviourAction<RequestReceived<T>, IntoHandler<T>>>,
    _ph: PhantomData<T>,
}

impl<T: Codec + Send + 'static> StreamingResponse<T> {
    pub fn new(config: StreamingResponseConfig) -> Self {
        Self {
            config,
            events: VecDeque::default(),
            requests: VecDeque::default(),
            _ph: PhantomData,
        }
    }

    pub fn request(&mut self, peer_id: PeerId, request: T::Request, channel: mpsc::Sender<Response<T::Response>>) {
        self.requests.push_back(NetworkBehaviourAction::NotifyHandler {
            peer_id,
            handler: NotifyHandler::Any,
            event: Request::new(request, channel),
        })
    }
}

impl<T: Codec + Send + 'static> NetworkBehaviour for StreamingResponse<T> {
    type ConnectionHandler = IntoHandler<T>;
    type OutEvent = RequestReceived<T>;

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        IntoHandler::new(
            self.config.max_message_size,
            self.config.request_timeout,
            self.config.response_send_buffer_size,
            self.config.keep_alive,
        )
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: <<Self::ConnectionHandler as libp2p::swarm::IntoConnectionHandler>::Handler as libp2p::swarm::ConnectionHandler>::OutEvent,
    ) {
        let handler::RequestReceived { request, channel } = event;
        tracing::trace!("request received by behaviour: {:?}", request);
        self.events.push_back(RequestReceived {
            peer_id,
            connection,
            request,
            channel,
        });
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
        _params: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        if let Some(action) = self.requests.pop_front() {
            tracing::trace!("triggering request action");
            return Poll::Ready(action);
        }
        match self.events.pop_front() {
            Some(e) => Poll::Ready(NetworkBehaviourAction::GenerateEvent(e)),
            None => Poll::Pending,
        }
    }
}
