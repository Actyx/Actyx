use actyxos_sdk::{
    service::{EventService, PublishRequest, PublishResponse},
    AppId,
};
use futures::{stream::BoxStream, FutureExt, StreamExt};
use wsrpc::Service;

pub struct Publish<S: EventService + Send> {
    event_service: S,
}

impl<S: EventService + Send + Sync + 'static> Service for Publish<S> {
    type Req = PublishRequest;
    type Resp = PublishResponse;
    type Error = ();
    type Ctx = AppId;

    fn serve(&self, _app_id: AppId, req: Self::Req) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        (async move { service.publish(req).await.map_err(|_| ()) })
            .into_stream()
            .boxed()
    }
}

pub fn service<S: EventService>(event_service: S) -> Publish<S> {
    Publish { event_service }
}
