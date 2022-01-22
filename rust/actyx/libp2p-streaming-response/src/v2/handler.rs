use super::{ProtocolError, Requester, Responder};
use crate::Codec;
use futures::{
    channel::mpsc, future::BoxFuture, stream::FuturesUnordered, AsyncReadExt, AsyncWriteExt, FutureExt, SinkExt,
    StreamExt,
};
use libp2p::swarm::{
    protocols_handler::{InboundUpgradeSend, OutboundUpgradeSend},
    IntoProtocolsHandler, NegotiatedSubstream, ProtocolsHandler, ProtocolsHandlerEvent, ProtocolsHandlerUpgrErr,
    SubstreamProtocol,
};
use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
    marker::PhantomData,
    task::{Context, Poll},
};

pub enum Command<T: Codec> {
    Request { id: u64, request: T::Request },
    Respond { id: u64, response: T::Response },
    Complete { id: u64 },
}

impl<T: Codec> Debug for Command<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request { id, request } => f
                .debug_struct("Send")
                .field("id", id)
                .field("request", request)
                .finish(),
            Self::Respond { id, response } => f
                .debug_struct("Respond")
                .field("id", id)
                .field("response", response)
                .finish(),
            Self::Complete { id } => f.debug_struct("Complete").field("id", id).finish(),
        }
    }
}

pub enum Event<T: Codec> {
    RequestSent(u64),
    RequestNotSent(u64, ProtocolsHandlerUpgrErr<ProtocolError>),
    RequestReceived(u64, T::Request),
    ResponseReceived(u64, T::Response),
    ResponseCompleted(u64),
}

impl<T: Codec> Debug for Event<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequestSent(arg0) => f.debug_tuple("RequestSent").field(arg0).finish(),
            Self::RequestNotSent(arg0, arg1) => f.debug_tuple("RequestNotSent").field(arg0).field(arg1).finish(),
            Self::RequestReceived(arg0, arg1) => f.debug_tuple("RequestReceived").field(arg0).field(arg1).finish(),
            Self::ResponseReceived(arg0, arg1) => f.debug_tuple("ResponseReceived").field(arg0).field(arg1).finish(),
            Self::ResponseCompleted(arg0) => f.debug_tuple("ResponseCompleted").field(arg0).finish(),
        }
    }
}

pub struct IntoHandler<T>(PhantomData<T>);

impl<T: Codec + Send + 'static> IntoProtocolsHandler for IntoHandler<T> {
    type Handler = Handler<T>;

    fn into_handler(
        self,
        _remote_peer_id: &libp2p::PeerId,
        _connected_point: &libp2p::core::ConnectedPoint,
    ) -> Self::Handler {
        Handler::new()
    }

    fn inbound_protocol(&self) -> <Self::Handler as libp2p::swarm::ProtocolsHandler>::InboundProtocol {
        Responder::new()
    }
}

type ProtocolEvent<T: Codec> = ProtocolsHandlerEvent<Requester<T>, u64, Event<T>, ProtocolError>;
type ResponseFuture = BoxFuture<'static, (u64, Result<(), ProtocolError>)>;

pub struct Handler<T: Codec> {
    events: VecDeque<ProtocolEvent<T>>,
    current_id: u64,
    stream_in: HashMap<u64, NegotiatedSubstream>,
    stream_out: HashMap<u64, mpsc::Sender<T::Response>>,
    streams: FuturesUnordered<ResponseFuture>,
    spawner: Box<dyn FnMut(ResponseFuture) -> ResponseFuture + Send + 'static>,
}

impl<T: Codec> Debug for Handler<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handler")
            .field("events", &self.events.len())
            .field("current_id", &self.current_id)
            .field("stream_in", &self.stream_in.keys().collect::<Vec<_>>())
            .field("stream_out", &self.stream_out.keys().collect::<Vec<_>>())
            .field("streams", &self.streams)
            .finish()
    }
}

impl<T: Codec> Handler<T> {
    pub fn new() -> Self {
        Self {
            events: VecDeque::default(),
            current_id: 0,
            stream_in: HashMap::default(),
            stream_out: HashMap::default(),
            streams: FuturesUnordered::default(),
            spawner: Box::new(|f| f),
        }
    }

    pub fn next_id(&mut self) -> u64 {
        self.current_id += 1;
        self.current_id
    }
}

impl<T: Codec + Send + 'static> ProtocolsHandler for Handler<T> {
    type InEvent = Command<T>;
    type OutEvent = Event<T>;
    type Error = ProtocolError;
    type InboundProtocol = Responder<T>;
    type OutboundProtocol = Requester<T>;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = u64;

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(Responder::new(), ())
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        protocol: <Self::InboundProtocol as InboundUpgradeSend>::Output,
        _info: Self::InboundOpenInfo,
    ) {
        let (request, mut stream) = protocol;
        let id = self.next_id();
        let (tx, mut rx) = mpsc::channel(128);
        let task = (self.spawner)(
            async move {
                while let Some(response) = rx.next().await {
                    let msg_bytes = serde_cbor::to_vec(&response)?;
                    let size_bytes = (msg_bytes.len() as u32).to_be_bytes();
                    stream.write_all(&size_bytes).await?;
                    stream.write_all(msg_bytes.as_slice()).await?;
                    stream.flush().await?;
                }
                stream.close().await?;
                Ok(())
            }
            .map(move |result| (id, result))
            .boxed(),
        );
        self.stream_out.insert(id, tx);
        self.streams.push(task);
        self.events
            .push_back(ProtocolsHandlerEvent::Custom(Event::RequestReceived(id, request)));
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        mut stream: <Self::OutboundProtocol as OutboundUpgradeSend>::Output,
        id: Self::OutboundOpenInfo,
    ) {
        let (mut tx, rx) = mpsc::channel(128);
        let task = (self.spawner)(
            async move {
                'outer: loop {
                    let mut size_bytes = [0u8; 4];
                    let mut to_read = &mut size_bytes[..];
                    while to_read.len() > 0 {
                        let read = stream.read(to_read).await?;
                        if read == 0 {
                            // stream closed
                            break 'outer;
                        }
                        to_read = to_read.split_at_mut(read).1;
                    }
                    let size = u32::from_be_bytes(size_bytes) as usize;
                    let mut msg_bytes = vec![0u8; size];
                    stream.read_exact(msg_bytes.as_mut_slice()).await?;
                    let msg = serde_cbor::from_slice(msg_bytes.as_slice())?;
                    tx.feed(msg).await?;
                }
                Ok(())
            }
            .map(|res| (0, res))
            .boxed(),
        );
        self.streams.push(task);
        self.events
            .push_back(ProtocolsHandlerEvent::Custom(Event::RequestSent(id)));
    }

    fn inject_event(&mut self, command: Self::InEvent) {
        match command {
            Command::Request { id, request } => {
                self.events.push_back(ProtocolsHandlerEvent::OutboundSubstreamRequest {
                    protocol: SubstreamProtocol::new(Requester::new(request), id),
                })
            }
            Command::Respond { id, response } => {}
            Command::Complete { id } => todo!(),
        }
    }

    fn inject_dial_upgrade_error(
        &mut self,
        info: Self::OutboundOpenInfo,
        error: ProtocolsHandlerUpgrErr<<Self::OutboundProtocol as OutboundUpgradeSend>::Error>,
    ) {
        self.events
            .push_back(ProtocolsHandlerEvent::Custom(Event::RequestNotSent(info, error)))
    }

    fn connection_keep_alive(&self) -> libp2p::swarm::KeepAlive {
        todo!()
    }

    fn poll(&mut self, cx: &mut Context<'_>) -> Poll<ProtocolEvent<T>> {
        match self.events.pop_front() {
            Some(e) => Poll::Ready(e),
            None => Poll::Pending,
        }
    }
}
