use super::{
    handler::{self, IntoHandler, Request, Response},
    RequestReceived, StreamingResponseConfig,
};
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
};

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
    type ProtocolsHandler = IntoHandler<T>;
    type OutEvent = RequestReceived<T>;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        IntoHandler::new(
            self.config.spawner.clone(),
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
        event: <<Self::ProtocolsHandler as libp2p::swarm::IntoProtocolsHandler>::Handler as libp2p::swarm::ProtocolsHandler>::OutEvent,
    ) {
        let handler::RequestReceived { request, channel } = event;
        log::trace!("request received by behaviour: {:?}", request);
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
    ) -> Poll<NetworkBehaviourAction<Self::OutEvent, Self::ProtocolsHandler>> {
        if let Some(action) = self.requests.pop_front() {
            log::trace!("triggering request action");
            return Poll::Ready(action);
        }
        match self.events.pop_front() {
            Some(e) => Poll::Ready(NetworkBehaviourAction::GenerateEvent(e)),
            None => Poll::Pending,
        }
    }
}
