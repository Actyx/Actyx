use super::{
    protocol::{Requester, Responder},
    ProtocolError,
};
use crate::Codec;
use futures::{
    channel::mpsc, executor::block_on, future::BoxFuture, stream::FuturesUnordered, AsyncReadExt, AsyncWriteExt,
    FutureExt, SinkExt, StreamExt,
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
    marker::PhantomData,
    task::{Context, Poll},
    time::Duration,
};

pub enum Response<T> {
    Msg(T),
    Error(ProtocolError),
    Finished,
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
    max_message_size: u32,
    request_timeout: Duration,
    response_send_buffer_size: usize,
    _ph: PhantomData<T>,
}

impl<T> IntoHandler<T> {
    pub fn new(max_message_size: u32, request_timeout: Duration, response_send_buffer_size: usize) -> Self {
        Self {
            max_message_size,
            request_timeout,
            response_send_buffer_size,
            _ph: PhantomData,
        }
    }
}

impl<T: Codec + Send + 'static> IntoProtocolsHandler for IntoHandler<T> {
    type Handler = Handler<T>;

    fn into_handler(self, _remote_peer_id: &PeerId, _connected_point: &ConnectedPoint) -> Self::Handler {
        Handler::new(
            self.max_message_size,
            self.request_timeout,
            self.response_send_buffer_size,
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
type ResponseFuture = BoxFuture<'static, Result<(), ProtocolError>>;

pub struct Handler<T: Codec> {
    events: VecDeque<ProtocolEvent<T>>,
    streams: FuturesUnordered<ResponseFuture>,
    spawner: Box<dyn FnMut(ResponseFuture) -> ResponseFuture + Send + 'static>,
    max_message_size: u32,
    request_timeout: Duration,
    response_send_buffer_size: usize,
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
    pub fn new(max_message_size: u32, request_timeout: Duration, response_send_buffer_size: usize) -> Self {
        Self {
            events: VecDeque::default(),
            streams: FuturesUnordered::default(),
            spawner: Box::new(|f| f),
            max_message_size,
            request_timeout,
            response_send_buffer_size,
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
        let task = (self.spawner)(
            async move {
                loop {
                    // only flush once we’re going to sleep
                    let response = match rx.try_next() {
                        Ok(Some(r)) => r,
                        Ok(None) => break,
                        Err(_) => {
                            stream.flush().await?;
                            match rx.next().await {
                                Some(r) => r,
                                None => break,
                            }
                        }
                    };
                    let msg_bytes = serde_cbor::to_vec(&response)?;
                    let size = msg_bytes.len();
                    if size > (max_message_size as usize) {
                        return Err(ProtocolError::MessageTooLargeSent(size));
                    }
                    let size_bytes = (size as u32).to_be_bytes();
                    stream.write_all(&size_bytes).await?;
                    stream.write_all(msg_bytes.as_slice()).await?;
                }
                stream.flush().await?;
                stream.close().await?;
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
                match async {
                    'outer: loop {
                        let mut size_bytes = [0u8; 4];
                        let mut to_read = &mut size_bytes[..];
                        while !to_read.is_empty() {
                            let read = stream.read(to_read).await?;
                            if read == 0 {
                                // stream closed
                                break 'outer;
                            }
                            to_read = to_read.split_at_mut(read).1;
                        }
                        let size = u32::from_be_bytes(size_bytes);

                        if size > max_message_size {
                            return Err(ProtocolError::MessageTooLargeRecv(size as usize));
                        }

                        let mut msg_bytes = vec![0u8; size as usize];
                        stream.read_exact(msg_bytes.as_mut_slice()).await?;
                        let msg = serde_cbor::from_slice(msg_bytes.as_slice())?;
                        tx.feed(Response::Msg(msg)).await?;
                    }
                    Ok(())
                }
                .await
                {
                    Ok(_) => tx.feed(Response::Finished).await?,
                    Err(e) => tx.feed(Response::Error(e)).await?,
                };
                Ok(())
            }
            .boxed(),
        );
        self.streams.push(task);
    }

    fn inject_event(&mut self, command: Self::InEvent) {
        let Request { request, channel } = command;
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
        block_on(tx.feed(Response::Error(error))).ok();
    }

    fn connection_keep_alive(&self) -> KeepAlive {
        if self.streams.is_empty() {
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
                    // we could also ignore (the substream has already been destroyed by dropping)
                    // not sure what is better
                    return Poll::Ready(ProtocolsHandlerEvent::Close(e));
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
