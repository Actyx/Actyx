// Recursive expansion of libp2p::NetworkBehaviour! macro
// =======================================================
use super::*;

impl<T1: Codec + Clone + Debug + Send + 'static, T2: Codec + Send + 'static> ::libp2p::swarm::NetworkBehaviour
    for StreamingResponse<T1, T2>
where
    v1::StreamingResponse<T1>: ::libp2p::swarm::NetworkBehaviour,
    Self: ::libp2p::swarm::NetworkBehaviourEventProcess<
        <v1::StreamingResponse<T1> as ::libp2p::swarm::NetworkBehaviour>::OutEvent,
    >,
    v2::StreamingResponse<T2>: ::libp2p::swarm::NetworkBehaviour,
    Self: ::libp2p::swarm::NetworkBehaviourEventProcess<
        <v2::StreamingResponse<T2> as ::libp2p::swarm::NetworkBehaviour>::OutEvent,
    >,
{
    type ProtocolsHandler = ::libp2p::swarm::IntoProtocolsHandlerSelect<
        <v1::StreamingResponse<T1> as ::libp2p::swarm::NetworkBehaviour>::ProtocolsHandler,
        <v2::StreamingResponse<T2> as ::libp2p::swarm::NetworkBehaviour>::ProtocolsHandler,
    >;
    type OutEvent = Output<T1, T2>;
    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        ::libp2p::swarm::IntoProtocolsHandler::select(self.v1.new_handler(), self.v2.new_handler())
    }
    fn addresses_of_peer(&mut self, peer_id: &::libp2p::core::PeerId) -> Vec<::libp2p::core::Multiaddr> {
        let mut out = Vec::new();
        out.extend(self.v1.addresses_of_peer(peer_id));
        out.extend(self.v2.addresses_of_peer(peer_id));
        out
    }
    fn inject_connected(&mut self, peer_id: &::libp2p::core::PeerId) {
        self.v1.inject_connected(peer_id);
        self.v2.inject_connected(peer_id);
    }
    fn inject_disconnected(&mut self, peer_id: &::libp2p::core::PeerId) {
        self.state.known_v2.remove(peer_id);
        self.v1.inject_disconnected(peer_id);
        self.v2.inject_disconnected(peer_id);
    }
    fn inject_connection_established(
        &mut self,
        peer_id: &::libp2p::core::PeerId,
        connection_id: &::libp2p::core::connection::ConnectionId,
        endpoint: &::libp2p::core::ConnectedPoint,
        errors: Option<&Vec<::libp2p::core::Multiaddr>>,
    ) {
        self.v1
            .inject_connection_established(peer_id, connection_id, endpoint, errors);
        self.v2
            .inject_connection_established(peer_id, connection_id, endpoint, errors);
    }
    fn inject_address_change(
        &mut self,
        peer_id: &::libp2p::core::PeerId,
        connection_id: &::libp2p::core::connection::ConnectionId,
        old: &::libp2p::core::ConnectedPoint,
        new: &::libp2p::core::ConnectedPoint,
    ) {
        self.v1.inject_address_change(peer_id, connection_id, old, new);
        self.v2.inject_address_change(peer_id, connection_id, old, new);
    }
    fn inject_connection_closed(
        &mut self,
        peer_id: &::libp2p::core::PeerId,
        connection_id: &::libp2p::core::connection::ConnectionId,
        endpoint: &::libp2p::core::ConnectedPoint,
        handlers: <Self::ProtocolsHandler as ::libp2p::swarm::IntoProtocolsHandler>::Handler,
    ) {
        let (handlers, handler) = handlers.into_inner();
        self.v2
            .inject_connection_closed(peer_id, connection_id, endpoint, handler);
        let handler = handlers;
        self.v1
            .inject_connection_closed(peer_id, connection_id, endpoint, handler);
    }
    fn inject_dial_failure(
        &mut self,
        peer_id: Option<::libp2p::core::PeerId>,
        handlers: Self::ProtocolsHandler,
        error: &::libp2p::swarm::DialError,
    ) {
        let (handlers, handler) = handlers.into_inner();
        self.v2.inject_dial_failure(peer_id, handler, error);
        let handler = handlers;
        self.v1.inject_dial_failure(peer_id, handler, error);
    }
    fn inject_listen_failure(
        &mut self,
        local_addr: &::libp2p::core::Multiaddr,
        send_back_addr: &::libp2p::core::Multiaddr,
        handlers: Self::ProtocolsHandler,
    ) {
        let (handlers, handler) = handlers.into_inner();
        self.v2.inject_listen_failure(local_addr, send_back_addr, handler);
        let handler = handlers;
        self.v1.inject_listen_failure(local_addr, send_back_addr, handler);
    }
    fn inject_new_listener(&mut self, id: ::libp2p::core::connection::ListenerId) {
        self.v1.inject_new_listener(id);
        self.v2.inject_new_listener(id);
    }
    fn inject_new_listen_addr(&mut self, id: ::libp2p::core::connection::ListenerId, addr: &::libp2p::core::Multiaddr) {
        self.v1.inject_new_listen_addr(id, addr);
        self.v2.inject_new_listen_addr(id, addr);
    }
    fn inject_expired_listen_addr(
        &mut self,
        id: ::libp2p::core::connection::ListenerId,
        addr: &::libp2p::core::Multiaddr,
    ) {
        self.v1.inject_expired_listen_addr(id, addr);
        self.v2.inject_expired_listen_addr(id, addr);
    }
    fn inject_new_external_addr(&mut self, addr: &::libp2p::core::Multiaddr) {
        self.v1.inject_new_external_addr(addr);
        self.v2.inject_new_external_addr(addr);
    }
    fn inject_expired_external_addr(&mut self, addr: &::libp2p::core::Multiaddr) {
        self.v1.inject_expired_external_addr(addr);
        self.v2.inject_expired_external_addr(addr);
    }
    fn inject_listener_error(
        &mut self,
        id: ::libp2p::core::connection::ListenerId,
        err: &(dyn std::error::Error + 'static),
    ) {
        self.v1.inject_listener_error(id, err);
        self.v2.inject_listener_error(id, err);
    }
    fn inject_listener_closed(
        &mut self,
        id: ::libp2p::core::connection::ListenerId,
        reason: std::result::Result<(), &std::io::Error>,
    ) {
        self.v1.inject_listener_closed(id, reason);
        self.v2.inject_listener_closed(id, reason);
    }
    fn inject_event(
        &mut self,
        peer_id: ::libp2p::core::PeerId,
        connection_id: ::libp2p::core::connection::ConnectionId,
        event: <<Self::ProtocolsHandler as ::libp2p::swarm::IntoProtocolsHandler> ::Handler as ::libp2p::swarm::ProtocolsHandler> ::OutEvent,
    ) {
        match event {
            ::libp2p::core::either::EitherOutput::First(ev) => {
                ::libp2p::swarm::NetworkBehaviour::inject_event(&mut self.v1, peer_id, connection_id, ev)
            }
            ::libp2p::core::either::EitherOutput::Second(ev) => {
                ::libp2p::swarm::NetworkBehaviour::inject_event(&mut self.v2, peer_id, connection_id, ev)
            }
        }
    }
    fn poll(
        &mut self,
        cx: &mut std::task::Context,
        poll_params: &mut impl ::libp2p::swarm::PollParameters,
    ) -> std::task::Poll<::libp2p::swarm::NetworkBehaviourAction<Self::OutEvent, Self::ProtocolsHandler>> {
        loop {
            match ::libp2p::swarm::NetworkBehaviour::poll(&mut self.v1, cx, poll_params) {
                std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::GenerateEvent(event)) => {
                    ::libp2p::swarm::NetworkBehaviourEventProcess::inject_event(self, event)
                }
                std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::Dial {
                    opts,
                    handler: provided_handler,
                }) => {
                    return std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::Dial {
                        opts,
                        handler: ::libp2p::swarm::IntoProtocolsHandler::select(provided_handler, self.v2.new_handler()),
                    });
                }
                std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler,
                    event,
                }) => {
                    return std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        handler,
                        event: ::libp2p::core::either::EitherOutput::First(event),
                    });
                }
                std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::ReportObservedAddr {
                    address,
                    score,
                }) => {
                    return std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::ReportObservedAddr {
                        address,
                        score,
                    });
                }
                std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::CloseConnection {
                    peer_id,
                    connection,
                }) => {
                    return std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::CloseConnection {
                        peer_id,
                        connection,
                    });
                }
                std::task::Poll::Pending => break,
            }
        }
        loop {
            match ::libp2p::swarm::NetworkBehaviour::poll(&mut self.v2, cx, poll_params) {
                std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::GenerateEvent(event)) => {
                    ::libp2p::swarm::NetworkBehaviourEventProcess::inject_event(self, event)
                }
                std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::Dial {
                    opts,
                    handler: provided_handler,
                }) => {
                    return std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::Dial {
                        opts,
                        handler: ::libp2p::swarm::IntoProtocolsHandler::select(self.v1.new_handler(), provided_handler),
                    });
                }
                std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::NotifyHandler {
                    peer_id,
                    handler,
                    event,
                }) => {
                    return std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::NotifyHandler {
                        peer_id,
                        handler,
                        event: ::libp2p::core::either::EitherOutput::Second(event),
                    });
                }
                std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::ReportObservedAddr {
                    address,
                    score,
                }) => {
                    return std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::ReportObservedAddr {
                        address,
                        score,
                    });
                }
                std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::CloseConnection {
                    peer_id,
                    connection,
                }) => {
                    return std::task::Poll::Ready(::libp2p::swarm::NetworkBehaviourAction::CloseConnection {
                        peer_id,
                        connection,
                    });
                }
                std::task::Poll::Pending => break,
            }
        }
        let f: std::task::Poll<::libp2p::swarm::NetworkBehaviourAction<Self::OutEvent, Self::ProtocolsHandler>> =
            StreamingResponse::poll_event(self, cx, poll_params);
        f
    }
}
