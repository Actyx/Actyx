use super::{
    protocol::{self, Requester, Responder},
    ProtocolError,
};
use crate::Codec;
use futures::{
    channel::mpsc, future::BoxFuture, stream::FuturesUnordered, AsyncWriteExt, FutureExt, SinkExt, StreamExt,
};
use libp2p::{
    core::{ConnectedPoint, UpgradeError},
    swarm::{
        protocols_handler::{InboundUpgradeSend, OutboundUpgradeSend},
        IntoProtocolsHandler, KeepAlive, ProtocolsHandler, ProtocolsHandlerEvent, ProtocolsHandlerUpgrErr,
        SubstreamProtocol,
    },
    PeerId,
};
use std::{
    collections::VecDeque,
    fmt::Debug,
    io::ErrorKind,
    marker::PhantomData,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

#[derive(Debug, PartialEq)]
pub enum Response<T> {
    Msg(T),
    Error(ProtocolError),
    Finished,
}

impl<T> Response<T> {
    pub fn into_msg(self) -> Result<T, ProtocolError> {
        match self {
            Response::Msg(msg) => Ok(msg),
            Response::Error(e) => Err(e),
            Response::Finished => Err(ProtocolError::Io(ErrorKind::UnexpectedEof.into())),
        }
    }
}

pub struct Request<T: Codec> {
    request: T::Request,
    channel: mpsc::Sender<Response<T::Response>>,
}

impl<T: Codec> Request<T> {
    pub fn new(request: T::Request, channel: mpsc::Sender<Response<T::Response>>) -> Self {
        Self { request, channel }
    }
}

impl<T: Codec> Debug for Request<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Request").field("request", &self.request).finish()
    }
}

pub struct RequestReceived<T: Codec> {
    pub(crate) request: T::Request,
    pub(crate) channel: mpsc::Sender<T::Response>,
}

impl<T: Codec> Debug for RequestReceived<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestReceived")
            .field("request", &self.request)
            .finish()
    }
}

pub struct IntoHandler<T> {
    spawner: Spawner,
    max_message_size: u32,
    request_timeout: Duration,
    response_send_buffer_size: usize,
    keep_alive: bool,
    _ph: PhantomData<T>,
}

impl<T> IntoHandler<T> {
    pub fn new(
        spawner: Spawner,
        max_message_size: u32,
        request_timeout: Duration,
        response_send_buffer_size: usize,
        keep_alive: bool,
    ) -> Self {
        Self {
            spawner,
            max_message_size,
            request_timeout,
            response_send_buffer_size,
            keep_alive,
            _ph: PhantomData,
        }
    }
}

impl<T: Codec + Send + 'static> IntoProtocolsHandler for IntoHandler<T> {
    type Handler = Handler<T>;

    fn into_handler(self, _remote_peer_id: &PeerId, _connected_point: &ConnectedPoint) -> Self::Handler {
        Handler::new(
            self.spawner,
            self.max_message_size,
            self.request_timeout,
            self.response_send_buffer_size,
            self.keep_alive,
        )
    }

    fn inbound_protocol(&self) -> <Self::Handler as ProtocolsHandler>::InboundProtocol {
        Responder::new(self.max_message_size)
    }
}

type ProtocolEvent<T> = ProtocolsHandlerEvent<
    Requester<T>,
    mpsc::Sender<Response<<T as Codec>::Response>>,
    RequestReceived<T>,
    ProtocolError,
>;
pub type ResponseFuture = BoxFuture<'static, Result<(), ProtocolError>>;
pub type Spawner = Arc<dyn Fn(ResponseFuture) -> ResponseFuture + Send + Sync + 'static>;

pub struct Handler<T: Codec> {
    events: VecDeque<ProtocolEvent<T>>,
    streams: FuturesUnordered<ResponseFuture>,
    spawner: Spawner,
    max_message_size: u32,
    request_timeout: Duration,
    response_send_buffer_size: usize,
    keep_alive: bool,
}

impl<T: Codec> Debug for Handler<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handler")
            .field("events", &self.events.len())
            .field("streams", &self.streams.len())
            .finish()
    }
}

impl<T: Codec> Handler<T> {
    pub fn new(
        spawner: Spawner,
        max_message_size: u32,
        request_timeout: Duration,
        response_send_buffer_size: usize,
        keep_alive: bool,
    ) -> Self {
        Self {
            events: VecDeque::default(),
            streams: FuturesUnordered::default(),
            spawner,
            max_message_size,
            request_timeout,
            response_send_buffer_size,
            keep_alive,
        }
    }
}

impl<T: Codec + Send + 'static> ProtocolsHandler for Handler<T> {
    type InEvent = Request<T>;
    type OutEvent = RequestReceived<T>;
    type Error = ProtocolError;
    type InboundProtocol = Responder<T>;
    type OutboundProtocol = Requester<T>;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = mpsc::Sender<Response<T::Response>>;

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(Responder::new(self.max_message_size), ()).with_timeout(self.request_timeout)
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        protocol: <Self::InboundProtocol as InboundUpgradeSend>::Output,
        _info: Self::InboundOpenInfo,
    ) {
        let (request, mut stream) = protocol;
        let (channel, mut rx) = mpsc::channel(self.response_send_buffer_size);
        let max_message_size = self.max_message_size;
        log::trace!("handler received request");
        let task = (self.spawner)(
            async move {
                log::trace!("starting send loop");
                loop {
                    // only flush once we’re going to sleep
                    let response = match rx.try_next() {
                        Ok(Some(r)) => r,
                        Ok(None) => break,
                        Err(_) => {
                            log::trace!("flushing stream");
                            stream.flush().await?;
                            match rx.next().await {
                                Some(r) => r,
                                None => break,
                            }
                        }
                    };
                    protocol::write_msg(&mut stream, response, max_message_size).await?;
                }
                log::trace!("flushing and closing substream");
                protocol::write_finish(&mut stream).await?;
                Ok(())
            }
            .boxed(),
        );
        self.streams.push(task);
        self.events
            .push_back(ProtocolsHandlerEvent::Custom(RequestReceived { request, channel }));
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        mut stream: <Self::OutboundProtocol as OutboundUpgradeSend>::Output,
        mut tx: Self::OutboundOpenInfo,
    ) {
        let max_message_size = self.max_message_size;
        let task = (self.spawner)(
            async move {
                log::trace!("starting receive loop");
                loop {
                    match protocol::read_msg(&mut stream, max_message_size)
                        .await
                        .unwrap_or_else(Response::Error)
                    {
                        Response::Msg(msg) => {
                            tx.feed(Response::Msg(msg)).await?;
                            log::trace!("response sent to client code");
                        }
                        Response::Error(e) => {
                            log::debug!("sending substream error {}", e);
                            tx.feed(Response::Error(e)).await?;
                            return Ok(());
                        }
                        Response::Finished => {
                            log::trace!("finishing substream");
                            tx.feed(Response::Finished).await?;
                            return Ok(());
                        }
                    }
                }
            }
            .boxed(),
        );
        self.streams.push(task);
    }

    fn inject_event(&mut self, command: Self::InEvent) {
        let Request { request, channel } = command;
        log::trace!("requesting {:?}", request);
        self.events.push_back(ProtocolsHandlerEvent::OutboundSubstreamRequest {
            protocol: SubstreamProtocol::new(Requester::new(self.max_message_size, request), channel)
                .with_timeout(self.request_timeout),
        })
    }

    fn inject_dial_upgrade_error(
        &mut self,
        mut tx: Self::OutboundOpenInfo,
        error: ProtocolsHandlerUpgrErr<<Self::OutboundProtocol as OutboundUpgradeSend>::Error>,
    ) {
        let error = match error {
            ProtocolsHandlerUpgrErr::Timeout => ProtocolError::Timeout,
            ProtocolsHandlerUpgrErr::Timer => ProtocolError::Timeout,
            ProtocolsHandlerUpgrErr::Upgrade(UpgradeError::Apply(e)) => e,
            ProtocolsHandlerUpgrErr::Upgrade(UpgradeError::Select(e)) => e.into(),
        };
        log::debug!("dial upgrade error: {}", error);
        if let Err(Response::Error(e)) = tx.try_send(Response::Error(error)).map_err(|e| e.into_inner()) {
            log::warn!("cannot send upgrade error to requester: {}", e);
        }
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        if !self.keep_alive && self.streams.is_empty() {
            KeepAlive::No
        } else {
            KeepAlive::Yes
        }
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ProtocolEvent<T>> {
        loop {
            if self.streams.is_empty() {
                break;
            }
            if let Poll::Ready(result) = self.streams.poll_next_unpin(cx) {
                // since the set was not empty, this must be a Some()
                if let Some(Err(e)) = result {
                    // no need to tear down the connection, substream is already closed
                    log::warn!("error in substream task: {}", e);
                }
            } else {
                break;
            }
        }

        match self.events.pop_front() {
            Some(e) => Poll::Ready(e),
            None => Poll::Pending,
        }
    }
}
