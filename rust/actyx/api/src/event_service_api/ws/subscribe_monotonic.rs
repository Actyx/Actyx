use actyxos_sdk::{
    service::{EventService, SubscribeMonotonicRequest, SubscribeMonotonicResponse},
    AppId,
};
use futures::{
    stream::{self, BoxStream},
    FutureExt, StreamExt,
};
use wsrpc::Service;

pub struct SubscribeMonotonic<S: EventService + Send> {
    event_service: S,
}

impl<S: EventService + Send + Sync + 'static> Service for SubscribeMonotonic<S> {
    type Req = SubscribeMonotonicRequest;
    type Resp = SubscribeMonotonicResponse;
    type Error = ();
    type Ctx = AppId;

    fn serve(&self, _app_id: AppId, req: Self::Req) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        (async move { service.subscribe_monotonic(req).await })
            .map(|x| match x {
                Ok(stream) => stream.map(Ok).left_stream(),
                Err(_) => stream::once(futures::future::err(())).right_stream(),
            })
            .flatten_stream()
            .boxed()
    }
}

pub fn service<S: EventService>(event_service: S) -> SubscribeMonotonic<S> {
    SubscribeMonotonic { event_service }
}
