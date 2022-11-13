use crate::{
    protocol::{RequestId, StreamingResponseConfig, StreamingResponseMessage},
    protocol_v2::{self, upgrade_inbound, upgrade_outbound, ProtocolError},
    upgrade::{from_fn, FromFnUpgrade},
    Codec, SequenceNo,
};
use futures::{
    channel::{mpsc, oneshot},
    future::{ready, select, BoxFuture, Either, Ready},
    stream::FuturesUnordered,
    AsyncWriteExt, FutureExt, SinkExt, StreamExt,
};
use libp2p::{
    core::{ConnectedPoint, Endpoint, UpgradeError},
    swarm::{
        handler::{InboundUpgradeSend, OutboundUpgradeSend},
        ConnectionHandler, ConnectionHandlerEvent, ConnectionHandlerUpgrErr, IntoConnectionHandler, KeepAlive,
        NegotiatedSubstream, SubstreamProtocol,
    },
    PeerId,
};
use smallvec::SmallVec;
use std::{
    collections::{BTreeMap, VecDeque},
    fmt::Debug,
    io::ErrorKind,
    marker::PhantomData,
    task::{Context, Poll},
    time::Duration,
};
use void::Void;

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
    max_message_size: u32,
    request_timeout: Duration,
    response_send_buffer_size: usize,
    keep_alive: bool,
    _ph: PhantomData<T>,
}

impl<T> IntoHandler<T> {
    pub fn new(
        max_message_size: u32,
        request_timeout: Duration,
        response_send_buffer_size: usize,
        keep_alive: bool,
    ) -> Self {
        Self {
            max_message_size,
            request_timeout,
            response_send_buffer_size,
            keep_alive,
            _ph: PhantomData,
        }
    }
}

impl<T: Codec + Send + 'static> IntoConnectionHandler for IntoHandler<T> {
    type Handler = Handler<T>;

    fn into_handler(self, _remote_peer_id: &PeerId, _connected_point: &ConnectedPoint) -> Self::Handler {
        Handler::new(
            self.max_message_size,
            self.request_timeout,
            self.response_send_buffer_size,
            self.keep_alive,
        )
    }

    fn inbound_protocol(&self) -> <Self::Handler as ConnectionHandler>::InboundProtocol {
        upgrade::<T>(false)
    }
}

fn upgrade<T: Codec>(only_v1: bool) -> Upgrade {
    if only_v1 {
        from_fn(T::protocol_info()[1..].into(), |stream, _endpoint, info| {
            ready(Ok((stream, info)))
        })
    } else {
        from_fn(T::protocol_info().into(), |stream, _endpoint, info| {
            ready(Ok((stream, info)))
        })
    }
}

type Upgrade = FromFnUpgrade<
    SmallVec<[&'static str; 2]>,
    fn(NegotiatedSubstream, Endpoint, &'static str) -> Ready<Result<(NegotiatedSubstream, &'static str), Void>>,
>;
type ProtocolEvent<T> = ConnectionHandlerEvent<
    Upgrade,
    <Handler<T> as ConnectionHandler>::OutboundOpenInfo,
    RequestReceived<T>,
    ProtocolError,
>;
pub type ResponseFuture = BoxFuture<'static, Result<(), ProtocolError>>;

pub struct Handler<T: Codec + Send + 'static> {
    events: VecDeque<ProtocolEvent<T>>,
    streams: FuturesUnordered<ResponseFuture>,
    inbound_v2: FuturesUnordered<BoxFuture<'static, Result<(T::Request, NegotiatedSubstream), ProtocolError>>>,
    inbound_v1: FuturesUnordered<<StreamingResponseConfig<T> as InboundUpgradeSend>::Future>,
    outbound_v1: FuturesUnordered<BoxFuture<'static, (RequestId, Result<(), ProtocolError>)>>,
    responses_v1: BTreeMap<RequestId, mpsc::Sender<Response<T::Response>>>,
    // cancellations coming from the peer, so NOT OUR REQUEST_IDs!
    cancel_v1: BTreeMap<RequestId, oneshot::Sender<()>>,
    v1_tx: mpsc::Sender<ProtocolEvent<T>>,
    v1_rx: mpsc::Receiver<ProtocolEvent<T>>,
    req_id: RequestId,
    max_message_size: u32,
    request_timeout: Duration,
    response_send_buffer_size: usize,
    keep_alive: bool,
}

impl<T: Codec + Send + 'static> Debug for Handler<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handler")
            .field("events", &self.events.len())
            .field("streams", &self.streams.len())
            .finish()
    }
}

impl<T: Codec + Send + 'static> Handler<T> {
    pub fn new(
        max_message_size: u32,
        request_timeout: Duration,
        response_send_buffer_size: usize,
        keep_alive: bool,
    ) -> Self {
        let (v1_tx, v1_rx) = mpsc::channel(response_send_buffer_size);
        Self {
            events: VecDeque::default(),
            streams: FuturesUnordered::default(),
            inbound_v2: FuturesUnordered::default(),
            inbound_v1: FuturesUnordered::default(),
            outbound_v1: FuturesUnordered::default(),
            responses_v1: BTreeMap::default(),
            cancel_v1: BTreeMap::default(),
            v1_tx,
            v1_rx,
            req_id: RequestId::default(),
            max_message_size,
            request_timeout,
            response_send_buffer_size,
            keep_alive,
        }
    }
}

pub enum OutboundInfo<T: Codec> {
    V1(StreamingResponseMessage<T>),
    V2(T::Request, mpsc::Sender<Response<T::Response>>),
}

impl<T: Codec + Send + 'static> ConnectionHandler for Handler<T> {
    type InEvent = Request<T>;
    type OutEvent = RequestReceived<T>;
    type Error = ProtocolError;
    type InboundProtocol = Upgrade;
    type OutboundProtocol = Upgrade;
    type InboundOpenInfo = ();
    type OutboundOpenInfo = OutboundInfo<T>;

    fn listen_protocol(&self) -> SubstreamProtocol<Self::InboundProtocol, Self::InboundOpenInfo> {
        SubstreamProtocol::new(upgrade::<T>(false), ()).with_timeout(self.request_timeout)
    }

    fn inject_fully_negotiated_inbound(
        &mut self,
        protocol: <Self::InboundProtocol as InboundUpgradeSend>::Output,
        _info: Self::InboundOpenInfo,
    ) {
        let (stream, proto) = protocol;
        tracing::trace!("handler received request for protocol {}", proto);
        if proto == T::info_v2() {
            // use the new stream-based approach
            self.inbound_v2
                .push(upgrade_inbound::<T>(self.max_message_size, stream, proto).boxed());
        } else if proto == T::info_v1() {
            // fall back to OneShot-based approach
            self.inbound_v1
                .push(StreamingResponseConfig::new(self.max_message_size as usize).upgrade_inbound(stream, proto));
        } else {
            tracing::error!(
                "inbound negotiation result `{}` is not among supported protocols [{}, {}], dropping stream",
                proto,
                T::info_v2(),
                T::info_v1(),
            );
        }
    }

    fn inject_fully_negotiated_outbound(
        &mut self,
        stream: <Self::OutboundProtocol as OutboundUpgradeSend>::Output,
        info: Self::OutboundOpenInfo,
    ) {
        let (stream, proto) = stream;
        tracing::trace!("handler opened outbound stream for protocol {}", proto);
        match info {
            OutboundInfo::V1(msg) => {
                self.streams.push(
                    async move {
                        let Err(err) = msg.upgrade_outbound(stream, T::info_v1()).await else { return Ok(()) };
                        tracing::debug!("outbound upgrade error on protocol `{}`: {}", T::info_v1(), err);
                        Ok(())
                    }
                    .boxed(),
                );
            }
            OutboundInfo::V2(request, mut tx) if proto == T::info_v2() => {
                let max_message_size = self.max_message_size;
                self.streams.push(
                    async move {
                        let result = upgrade_outbound::<T>(max_message_size, request, stream, T::info_v2()).await;
                        let mut stream = match result {
                            Ok(stream) => stream,
                            Err(err) => {
                                // assuming that the response channel has at least capacity 1
                                tx.try_send(Response::Error(err)).ok();
                                return Ok(());
                            }
                        };
                        tracing::trace!("starting receive loop for protocol `{}`", T::info_v2());
                        let mut buffer = Vec::new();
                        loop {
                            match protocol_v2::read_msg(&mut stream, max_message_size, &mut buffer)
                                .await
                                .unwrap_or_else(Response::Error)
                            {
                                Response::Msg(msg) => {
                                    tx.feed(Response::Msg(msg)).await?;
                                    tracing::trace!("response sent to client code");
                                }
                                Response::Error(e) => {
                                    tracing::debug!("sending substream error {}", e);
                                    tx.feed(Response::Error(e)).await?;
                                    return Ok(());
                                }
                                Response::Finished => {
                                    tracing::trace!("finishing substream");
                                    tx.feed(Response::Finished).await?;
                                    return Ok(());
                                }
                            }
                        }
                    }
                    .boxed(),
                );
            }
            OutboundInfo::V2(request, tx) if proto == T::info_v1() => {
                let request_id = self.req_id;
                self.req_id.increment();
                self.responses_v1.insert(request_id, tx);
                self.outbound_v1.push(
                    StreamingResponseMessage::<T>::Request {
                        id: request_id,
                        payload: request,
                    }
                    .upgrade_outbound(stream, T::info_v1())
                    .map(move |x| (request_id, x.map_err(ProtocolError::Io)))
                    .boxed(),
                )
            }
            OutboundInfo::V2(_, _) => {
                tracing::error!(
                    "inbound negotiation result `{}` is not among supported protocols [{}, {}], dropping stream",
                    proto,
                    T::info_v2(),
                    T::info_v1(),
                );
            }
        }
    }

    fn inject_event(&mut self, command: Self::InEvent) {
        let Request { request, channel } = command;
        tracing::trace!("requesting {:?}", request);
        self.events.push_back(ConnectionHandlerEvent::OutboundSubstreamRequest {
            protocol: SubstreamProtocol::new(upgrade::<T>(false), OutboundInfo::V2(request, channel))
                .with_timeout(self.request_timeout),
        })
    }

    fn inject_dial_upgrade_error(
        &mut self,
        info: Self::OutboundOpenInfo,
        error: ConnectionHandlerUpgrErr<<Self::OutboundProtocol as OutboundUpgradeSend>::Error>,
    ) {
        let error = match error {
            ConnectionHandlerUpgrErr::Timeout => ProtocolError::Timeout,
            ConnectionHandlerUpgrErr::Timer => ProtocolError::Timeout,
            ConnectionHandlerUpgrErr::Upgrade(UpgradeError::Apply(e)) => void::unreachable(e),
            ConnectionHandlerUpgrErr::Upgrade(UpgradeError::Select(e)) => e.into(),
        };
        tracing::debug!("dial upgrade error: {}", error);
        if let OutboundInfo::V2(_, mut tx) = info {
            if let Err(Response::Error(e)) = tx.try_send(Response::Error(error)).map_err(|e| e.into_inner()) {
                tracing::warn!("cannot send upgrade error to requester: {}", e);
            }
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
        while !self.inbound_v2.is_empty() {
            let Poll::Ready(Some(result)) = self.inbound_v2.poll_next_unpin(cx) else { break };
            match result {
                Ok((request, mut stream)) => {
                    let (channel, mut rx) = mpsc::channel(self.response_send_buffer_size);
                    let max_message_size = self.max_message_size;
                    self.streams.push(
                        async move {
                            tracing::trace!("starting send loop");
                            let mut buffer = Vec::new();
                            loop {
                                // only flush once we’re going to sleep
                                let response = match rx.try_next() {
                                    Ok(Some(r)) => r,
                                    Ok(None) => break,
                                    Err(_) => {
                                        tracing::trace!("flushing stream");
                                        stream.flush().await?;
                                        match rx.next().await {
                                            Some(r) => r,
                                            None => break,
                                        }
                                    }
                                };
                                protocol_v2::write_msg(&mut stream, response, max_message_size, &mut buffer).await?;
                            }
                            tracing::trace!("flushing and closing substream");
                            protocol_v2::write_finish(&mut stream).await?;
                            Ok(())
                        }
                        .boxed(),
                    );
                    self.events
                        .push_back(ConnectionHandlerEvent::Custom(RequestReceived { request, channel }));
                }
                Err(err) => tracing::debug!("inbound upgrade error for protocol `{}`: {}", T::info_v2(), err),
            }
        }

        while !self.inbound_v1.is_empty() {
            let Poll::Ready(Some(result)) = self.inbound_v1.poll_next_unpin(cx) else { break };
            match result {
                Ok(request) => match request {
                    StreamingResponseMessage::Request { id, payload } => {
                        let mut tx = self.v1_tx.clone();
                        let (channel, mut rx) = mpsc::channel(self.response_send_buffer_size);
                        let (cancel_tx, mut cancel_rx) = oneshot::channel();
                        self.cancel_v1.insert(id, cancel_tx);
                        self.streams.push(
                            async move {
                                let mut seq_no = SequenceNo(0);
                                while let Either::Left((Some(payload), _)) = select(rx.next(), &mut cancel_rx).await {
                                    seq_no.increment();
                                    tx.send(ConnectionHandlerEvent::OutboundSubstreamRequest {
                                        protocol: SubstreamProtocol::new(
                                            upgrade::<T>(true),
                                            OutboundInfo::V1(StreamingResponseMessage::Response {
                                                id,
                                                seq_no,
                                                payload,
                                            }),
                                        ),
                                    })
                                    .await?;
                                }
                                seq_no.increment();
                                tx.send(ConnectionHandlerEvent::OutboundSubstreamRequest {
                                    protocol: SubstreamProtocol::new(
                                        upgrade::<T>(true),
                                        OutboundInfo::V1(StreamingResponseMessage::ResponseEnd { id, seq_no }),
                                    ),
                                })
                                .await?;
                                Ok(())
                            }
                            .boxed(),
                        );
                        self.events.push_back(ConnectionHandlerEvent::Custom(RequestReceived {
                            request: payload,
                            channel,
                        }));
                    }
                    StreamingResponseMessage::CancelRequest { id } => {
                        if let Some(tx) = self.cancel_v1.remove(&id) {
                            tx.send(()).ok();
                        } else {
                            tracing::debug!("`{}` dropping cancellation for unknown request", T::info_v1());
                        }
                    }
                    StreamingResponseMessage::Response { id, seq_no: _, payload } => {
                        if let Some(tx) = self.responses_v1.get_mut(&id) {
                            if let Err(err) = tx.try_send(Response::Msg(payload)) {
                                if err.is_disconnected() {
                                    self.events.push_back(ConnectionHandlerEvent::OutboundSubstreamRequest {
                                        protocol: SubstreamProtocol::new(
                                            upgrade::<T>(true),
                                            OutboundInfo::V1(StreamingResponseMessage::CancelRequest { id }),
                                        ),
                                    });
                                    self.responses_v1.remove(&id);
                                }
                                tracing::warn!("`{}` dropping response: {}", T::info_v1(), err);
                            }
                        } else {
                            tracing::debug!("`{}` dropping response for unknown request", T::info_v1());
                        }
                    }
                    StreamingResponseMessage::ResponseEnd { id, seq_no: _ } => {
                        if let Some(mut tx) = self.responses_v1.remove(&id) {
                            if let Err(err) = tx.try_send(Response::Finished) {
                                tracing::warn!("`{}` dropping response end: {}", T::info_v1(), err);
                            }
                        } else {
                            tracing::debug!("`{}` dropping response for unknown request", T::info_v1());
                        }
                    }
                },
                Err(err) => tracing::debug!("inbound upgrade error for protocol `{}`: {}", T::info_v1(), err),
            }
        }

        while let Poll::Ready(Some(msg)) = self.v1_rx.poll_next_unpin(cx) {
            self.events.push_back(msg);
        }

        if let Some(e) = self.events.pop_front() {
            return Poll::Ready(e);
        }

        while !self.outbound_v1.is_empty() {
            let Poll::Ready(Some((request_id, result))) = self.outbound_v1.poll_next_unpin(cx) else { break };
            if let Err(e) = result {
                tracing::debug!("error in v1 substream task: {}", e);
                if let Some(mut tx) = self.responses_v1.remove(&request_id) {
                    tx.try_send(Response::Error(e)).ok();
                }
            }
        }

        let mut some_finished = false;
        while !self.streams.is_empty() {
            let Poll::Ready(Some(result)) = self.streams.poll_next_unpin(cx) else { break };
            some_finished = true;
            if let Err(e) = result {
                tracing::debug!("error in substream task: {}", e);
            }
        }
        if some_finished {
            self.cancel_v1.retain(|_k, v| !v.is_canceled());
        }

        Poll::Pending
    }
}
