#![allow(clippy::new_without_default)]
#![allow(clippy::return_self_not_must_use)]

mod behaviour;
pub mod v1;
pub mod v2;

use futures::{
    channel::mpsc, future::BoxFuture, stream::FuturesUnordered, Future, FutureExt, SinkExt, StreamExt, TryFutureExt,
};
use libp2p::{
    core::{connection::ConnectionId, upgrade::NegotiationError},
    swarm::{NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters},
    PeerId,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    any::Any,
    collections::{HashMap, VecDeque},
    fmt::Debug,
    io::ErrorKind,
    task::{Context, Poll},
};
use v2::{Response, Spawner, StreamingResponseConfig};

/// A [`Codec`] defines the request and response types for a [`StreamingResponse`]
/// protocol. Request and responses are encoded / decoded using `serde_cbor`, so
/// `Serialize` and `Deserialize` impls have to be provided. Implement this trait
/// to specialize the [`StreamingResponse`].
pub trait Codec {
    type Request: Send + Serialize + DeserializeOwned + std::fmt::Debug + 'static;
    type Response: Send + Serialize + DeserializeOwned + std::fmt::Debug;

    fn protocol_info() -> &'static [u8];
}

// #[derive(libp2p::NetworkBehaviour)]
// #[behaviour(out_event = "Output<T1, T2>", event_process = true, poll_method = "poll_event")]
pub struct StreamingResponse<T1: Codec + Clone + Debug + Send + 'static, T2: Codec + Send + 'static> {
    v1: v1::StreamingResponse<T1>,
    v2: v2::StreamingResponse<T2>,
    // #[behaviour(ignore)]
    state: State<T1, T2>,
}

#[derive(Debug)]
pub enum Output<T1: Codec, T2: Codec> {
    V1(RequestReceived<T1>),
    V2(RequestReceived<T2>),
}

impl<T1: Codec, T2: Codec> Output<T1, T2> {
    pub fn peer_id(&self) -> PeerId {
        match self {
            Output::V1(r) => r.peer_id,
            Output::V2(r) => r.peer_id,
        }
    }

    pub fn connection(&self) -> ConnectionId {
        match self {
            Output::V1(r) => r.connection,
            Output::V2(r) => r.connection,
        }
    }

    pub fn feeder(&self) -> Feeder<T1::Response, T2::Response> {
        match self {
            Output::V1(r) => Feeder::V1(r.channel.clone()),
            Output::V2(r) => Feeder::V2(r.channel.clone()),
        }
    }
}

pub enum Feeder<T1, T2> {
    V1(mpsc::Sender<T1>),
    V2(mpsc::Sender<T2>),
}

impl<T1: Send + 'static, T2: Send + 'static> Feeder<T1, T2> {
    pub fn feed(&mut self, response: T2) -> impl Future<Output = Result<(), SendError>> + Send + '_
    where
        T2: Into<T1>,
    {
        match self {
            Self::V1(r) => r.feed(response.into()).map_err(|_| SendError).left_future(),
            Self::V2(r) => r.feed(response).map_err(|_| SendError).right_future(),
        }
    }
}

#[derive(Debug)]
pub struct SendError;

#[derive(Debug)]
pub struct RequestReceived<T: Codec> {
    pub peer_id: PeerId,
    pub connection: ConnectionId,
    pub request: T::Request,
    pub channel: mpsc::Sender<T::Response>,
}

pub enum EitherResponse<T1, T2> {
    V1(T1),
    V2(T2),
}
impl<T> EitherResponse<T, T> {
    pub fn squash(self) -> T {
        match self {
            EitherResponse::V1(t) => t,
            EitherResponse::V2(t) => t,
        }
    }
}

enum Action<T1: Codec, T2: Codec> {
    DoneV1,
    DoneV2(PeerId),
    RetryV1(
        PeerId,
        T1::Request,
        mpsc::Sender<Response<T2::Response>>,
        fn(T1::Response) -> T2::Response,
    ),
    Error(v2::ProtocolError),
}

type MyHandler<T1, T2> = <StreamingResponse<T1, T2> as NetworkBehaviour>::ProtocolsHandler;
struct State<T1: Codec + Clone + Debug + Send + 'static, T2: Codec + Send + 'static> {
    config: StreamingResponseConfig,
    tasks: FuturesUnordered<BoxFuture<'static, Option<Action<T1, T2>>>>,
    events: VecDeque<NetworkBehaviourAction<Output<T1, T2>, MyHandler<T1, T2>>>,
    known_v2: HashMap<PeerId, bool>,
    request_v1: HashMap<v1::RequestId, mpsc::UnboundedSender<v1::StreamingResponseEvent<T1>>>,
    response_v1: mpsc::Sender<(v1::ChannelId, Response<T1::Response>)>,
    poll_v1: mpsc::Receiver<(v1::ChannelId, Response<T1::Response>)>,
}

impl<T1: Codec + Clone + Debug + Send + 'static, T2: Codec + Send + 'static> Default for State<T1, T2> {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(128);
        Self {
            config: Default::default(),
            tasks: Default::default(),
            events: Default::default(),
            known_v2: Default::default(),
            request_v1: Default::default(),
            response_v1: tx,
            poll_v1: rx,
        }
    }
}

impl<T1: Codec + Clone + Debug + Send + 'static, T2: Codec + Send + 'static> StreamingResponse<T1, T2> {
    pub fn new(config: StreamingResponseConfig) -> Self {
        let v1 = v1::StreamingResponseConfig {
            max_buf_size: config.response_send_buffer_size,
            ..v1::StreamingResponseConfig::default()
        };
        Self {
            v1: v1::StreamingResponse::new(v1),
            v2: v2::StreamingResponse::new(config),
            state: State::default(),
        }
    }
    pub fn request<T>(&mut self, peer_id: PeerId, request: T, mut channel: mpsc::Sender<Response<T2::Response>>)
    where
        T1::Response: Into<T2::Response>,
        T: Clone + Into<T1::Request> + Into<T2::Request> + Send + 'static,
    {
        if self.state.known_v2.get(&peer_id).copied() == Some(false) {
            self.request_v1(peer_id, request.into(), channel, Into::into);
            return;
        }

        let (tx, mut rx) = mpsc::channel(self.state.config.response_send_buffer_size);
        self.v2.request(peer_id, request.clone().into(), tx);
        self.state.tasks.push(spawn(
            &self.state.config.spawner,
            async move {
                while let Some(ev) = rx.next().await {
                    match ev {
                        Response::Error(v2::ProtocolError::Negotiation(NegotiationError::Failed)) => {
                            return Ok(Action::RetryV1(
                                peer_id,
                                Into::<T1::Request>::into(request),
                                channel,
                                Into::into,
                            ))
                        }
                        msg => channel.feed(msg).await?,
                    }
                }
                Ok(Action::DoneV2(peer_id))
            }
            .map(|res| match res {
                Ok(msg) => msg,
                Err(e) => Action::Error(e),
            }),
        ))
    }
    pub fn request_v1(
        &mut self,
        peer_id: PeerId,
        request: T1::Request,
        mut channel: mpsc::Sender<Response<T2::Response>>,
        into: fn(T1::Response) -> T2::Response,
    ) {
        use v1::StreamingResponseEvent::*;
        let id = self.v1.request(peer_id, request);
        let (tx, mut rx) = mpsc::unbounded();
        self.state.request_v1.insert(id, tx);
        self.state.tasks.push(spawn(
            &self.state.config.spawner,
            async move {
                while let Some(ev) = rx.next().await {
                    match ev {
                        CancelledRequest { .. } => {
                            channel
                                .feed(Response::Error(v2::ProtocolError::Io(std::io::Error::new(
                                    ErrorKind::Other,
                                    "connection closed",
                                ))))
                                .await?;
                        }
                        ResponseReceived { payload, .. } => {
                            channel.feed(Response::Msg(into(payload))).await?;
                        }
                        ResponseFinished { .. } => {
                            channel.feed(Response::Finished).await?;
                        }
                        _ => {}
                    }
                }
                Ok(Action::DoneV1)
            }
            .map(|res| match res {
                Ok(msg) => msg,
                Err(e) => Action::Error(e),
            }),
        ));
    }
    fn poll_event(
        &mut self,
        cx: &mut Context,
        _: &mut impl PollParameters,
    ) -> Poll<NetworkBehaviourAction<Output<T1, T2>, <Self as NetworkBehaviour>::ProtocolsHandler>> {
        while let Ok(Some((ch_id, resp))) = self.state.poll_v1.try_next() {
            match resp {
                Response::Msg(msg) => self.v1.respond(ch_id, msg),
                Response::Error(_) => self.v1.finish_response(ch_id),
                Response::Finished => self.v1.finish_response(ch_id),
            }
            .ok();
        }
        while !self.state.tasks.is_empty() {
            match self.state.tasks.poll_next_unpin(cx) {
                Poll::Ready(Some(Some(Action::DoneV1))) => {}
                Poll::Ready(Some(Some(Action::DoneV2(peer_id)))) => {
                    self.state.known_v2.insert(peer_id, true);
                }
                Poll::Ready(Some(Some(Action::RetryV1(peer_id, request, channel, into)))) => {
                    self.request_v1(peer_id, request, channel, into);
                }
                Poll::Ready(Some(Some(Action::Error(e)))) => log::warn!("task failed: {}", e),
                Poll::Ready(Some(None)) => log::warn!("task join error"),
                Poll::Ready(None) => unreachable!(),
                Poll::Pending => break,
            }
        }
        if let Some(ev) = self.state.events.pop_front() {
            return Poll::Ready(ev);
        }
        Poll::Pending
    }
}

impl<T1: Codec + Clone + Debug + Send + 'static, T2: Codec + Send + 'static>
    NetworkBehaviourEventProcess<v1::StreamingResponseEvent<T1>> for StreamingResponse<T1, T2>
{
    fn inject_event(&mut self, event: v1::StreamingResponseEvent<T1>) {
        match event {
            v1::StreamingResponseEvent::ReceivedRequest { channel_id, payload } => {
                let (tx, mut rx) = mpsc::channel(self.state.config.response_send_buffer_size);
                let event = RequestReceived {
                    peer_id: channel_id.peer(),
                    connection: channel_id.connection(),
                    request: payload,
                    channel: tx,
                };
                let mut responses = self.state.response_v1.clone();
                self.state.tasks.push(spawn(
                    &self.state.config.spawner,
                    async move {
                        while let Some(msg) = rx.next().await {
                            responses.feed((channel_id, Response::Msg(msg))).await?;
                        }
                        responses.feed((channel_id, Response::Finished)).await?;
                        Ok(Action::DoneV1)
                    }
                    .map(|res| match res {
                        Ok(msg) => msg,
                        Err(e) => Action::Error(e),
                    }),
                ));
                self.state
                    .events
                    .push_back(NetworkBehaviourAction::GenerateEvent(Output::V1(event)))
            }
            msg => {
                if let Some(tx) = self.state.request_v1.get_mut(&msg.request_id()) {
                    tx.unbounded_send(msg).ok();
                }
            }
        }
    }
}

impl<T1: Codec + Clone + Debug + Send + 'static, T2: Codec + Send + 'static>
    NetworkBehaviourEventProcess<v2::RequestReceived<T2>> for StreamingResponse<T1, T2>
{
    fn inject_event(&mut self, event: v2::RequestReceived<T2>) {
        let event = RequestReceived {
            peer_id: event.peer_id,
            connection: event.connection,
            request: event.request,
            channel: event.channel,
        };
        self.state
            .events
            .push_back(NetworkBehaviourAction::GenerateEvent(Output::V2(event)))
    }
}

fn spawn<T: Send + 'static>(
    spawner: &Spawner,
    f: impl Future<Output = T> + Send + 'static,
) -> BoxFuture<'static, Option<T>> {
    (spawner)(f.map(|t| -> Box<dyn Any + Send + 'static> { Box::new(t) }).boxed())
        .map(|b| if let Ok(t) = b.downcast::<T>() { Some(*t) } else { None })
        .boxed()
}
