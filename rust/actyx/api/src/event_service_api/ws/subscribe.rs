use actyxos_sdk::{
    event_service::{EventService, SubscribeRequest, SubscribeResponse},
    tagged::AppId,
};
use futures::{
    stream::{self, BoxStream},
    FutureExt, StreamExt,
};
use wsrpc::Service;

pub struct Subscribe<S: EventService + Send> {
    event_service: S,
}

impl<S: EventService + Send + Sync + 'static> Service for Subscribe<S> {
    type Req = SubscribeRequest;
    type Resp = SubscribeResponse;
    type Error = ();
    type Ctx = AppId;

    fn serve(&self, _app_id: AppId, req: Self::Req) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        async move {
            service
                .subscribe(req)
                .map(move |x| match x {
                    Ok(stream) => stream.map(Ok).left_stream(),
                    Err(_) => stream::once(futures::future::err(())).right_stream(),
                })
                .await
        }
        .flatten_stream()
        .boxed()
    }
}

pub fn service<S: EventService>(event_service: S) -> Subscribe<S> {
    Subscribe { event_service }
}
