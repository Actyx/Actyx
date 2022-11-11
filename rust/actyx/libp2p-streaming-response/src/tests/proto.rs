use futures::{
    future::{ready, Ready},
    AsyncWriteExt,
};
use libp2p::{
    core::UpgradeInfo,
    swarm::{
        handler::{InboundUpgradeSend, OutboundUpgradeSend},
        ConnectionHandler, ConnectionHandlerEvent, ConnectionHandlerUpgrErr, IntoConnectionHandler, KeepAlive,
        NegotiatedSubstream, NetworkBehaviour, SubstreamProtocol,
    },
    InboundUpgrade, OutboundUpgrade,
};
use std::{
    convert::Infallible,
    iter::{once, Once},
    task::{Context, Poll},
};
use tokio::runtime::Handle;

pub struct TestBehaviour(pub Handle, pub Vec<u8>);

impl NetworkBehaviour for TestBehaviour {
    type ConnectionHandler = TestHandler;
    type OutEvent = ();

    fn new_handler(&mut self) -> Self::ConnectionHandler {
        TestHandler(self.0.clone(), self.1.clone())
    }

    fn inject_event(
        &mut self,
        _peer_id: libp2p::PeerId,
        _connection: libp2p::core::connection::ConnectionId,
        _event: <<Self::ConnectionHandler as IntoConnectionHandler>::Handler as ConnectionHandler>::OutEvent,
    ) {
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
        _params: &mut impl libp2p::swarm::PollParameters,
    ) -> Poll<libp2p::swarm::NetworkBehaviourAction<Self::OutEvent, Self::ConnectionHandler>> {
        Poll::Pending
    }
}

pub struct TestHandler(Handle, Vec<u8>);

impl ConnectionHandler for TestHandler {
    type InEvent = ();
    type OutEvent = ();
    type Error = Infallible;
    type InboundProtocol = Proto;
    type OutboundProtocol = Proto;
    type InboundOpenInfo = Vec<u8>;
    type OutboundOpenInfo = ();

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(Proto, self.1.clone())
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        mut socket: <Self::InboundProtocol as InboundUpgradeSend>::Output,
        bytes: Self::InboundOpenInfo,
    ) {
        tracing::trace!("inbound negotiated");
        self.0.spawn(async move {
            tracing::trace!("sending fake bytes");
            socket.write_all(&bytes).await?;
            socket.flush().await?;
            socket.close().await?;
            Result::<_, std::io::Error>::Ok(())
        });
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        _protocol: <Self::OutboundProtocol as OutboundUpgradeSend>::Output,
        _info: Self::OutboundOpenInfo,
    ) {
    }

    fn inject_event(&mut self, _event: Self::InEvent) {}

    fn inject_dial_upgrade_error(
        &mut self,
        _info: Self::OutboundOpenInfo,
        _error: ConnectionHandlerUpgrErr<<Self::OutboundProtocol as OutboundUpgradeSend>::Error>,
    ) {
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        KeepAlive::Yes
    }

    fn poll(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<ConnectionHandlerEvent<Self::OutboundProtocol, Self::OutboundOpenInfo, Self::OutEvent, Self::Error>> {
        Poll::Pending
    }
}

pub struct Proto;

impl UpgradeInfo for Proto {
    type Info = &'static [u8];
    type InfoIter = Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        once(super::PROTO_V2.as_bytes())
    }
}

impl InboundUpgrade<NegotiatedSubstream> for Proto {
    type Output = NegotiatedSubstream;
    type Error = ();
    type Future = Ready<Result<NegotiatedSubstream, ()>>;

    fn upgrade_inbound(self, socket: NegotiatedSubstream, _info: Self::Info) -> Self::Future {
        tracing::debug!("got inbound");
        ready(Ok(socket))
    }
}

impl OutboundUpgrade<NegotiatedSubstream> for Proto {
    type Output = NegotiatedSubstream;
    type Error = ();
    type Future = Ready<Result<NegotiatedSubstream, ()>>;

    fn upgrade_outbound(self, socket: NegotiatedSubstream, _info: Self::Info) -> Self::Future {
        tracing::debug!("got outbound");
        ready(Ok(socket))
    }
}
