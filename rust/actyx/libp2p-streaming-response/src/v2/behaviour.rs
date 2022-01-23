use super::handler::{IntoHandler, Request, RequestReceived, Response};
use crate::Codec;
use futures::channel::mpsc;
use libp2p::{
    core::connection::ConnectionId,
    swarm::{NetworkBehaviour, NetworkBehaviourAction, NotifyHandler, PollParameters},
    PeerId,
};
use std::{
    collections::VecDeque,
    marker::PhantomData,
    task::{Context, Poll},
    time::Duration,
};

pub enum Event<T: Codec> {
    RequestReceived {
        peer_id: PeerId,
        connection: ConnectionId,
        request: T::Request,
        channel: mpsc::Sender<T::Response>,
    },
}

pub struct StreamingResponseConfig {
    request_timeout: Duration,
    max_message_size: u32,
    response_send_buffer_size: usize,
}

impl StreamingResponseConfig {
    pub fn with_request_timeout(self, request_timeout: Duration) -> Self {
        Self {
            request_timeout,
            ..self
        }
    }
}

impl Default for StreamingResponseConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(10),
            max_message_size: 1_000_000,
            response_send_buffer_size: 128,
        }
    }
}

pub struct StreamingResponse<T: Codec + Send + 'static> {
    config: StreamingResponseConfig,
    events: VecDeque<Event<T>>,
    requests: VecDeque<NetworkBehaviourAction<Event<T>, IntoHandler<T>>>,
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
    type ProtocolsHandler = IntoHandler<T>;
    type OutEvent = Event<T>;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        IntoHandler::new(
            self.config.max_message_size,
            self.config.request_timeout,
            self.config.response_send_buffer_size,
        )
    }

    fn inject_event(
        &mut self,
        peer_id: PeerId,
        connection: ConnectionId,
        event: <<Self::ProtocolsHandler as libp2p::swarm::IntoProtocolsHandler>::Handler as libp2p::swarm::ProtocolsHandler>::OutEvent,
    ) {
        let RequestReceived { request, channel } = event;
        self.events.push_back(Event::RequestReceived {
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
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ProtocolsHandler>> {
        match self.events.pop_front() {
            Some(e) => Poll::Ready(NetworkBehaviourAction::GenerateEvent(e)),
            None => Poll::Pending,
        }
    }
}
