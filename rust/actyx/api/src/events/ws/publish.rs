use actyxos_sdk::{
    service::{PublishRequest, PublishResponse},
    AppId,
};
use futures::{stream::BoxStream, FutureExt, StreamExt};
use wsrpc::Service;

use crate::events::service::EventService;

pub struct Publish {
    event_service: EventService,
}

impl Service for Publish {
    type Req = PublishRequest;
    type Resp = PublishResponse;
    type Error = ();
    type Ctx = AppId;

    fn serve(&self, app_id: AppId, req: Self::Req) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        (async move { service.publish(app_id, req).await.map_err(|_| ()) })
            .into_stream()
            .boxed()
    }
}

pub fn service(event_service: EventService) -> Publish {
    Publish { event_service }
}
