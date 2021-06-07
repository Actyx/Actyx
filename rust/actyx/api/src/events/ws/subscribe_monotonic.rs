use actyxos_sdk::{
    service::{SubscribeMonotonicRequest, SubscribeMonotonicResponse},
    AppId,
};
use futures::{
    stream::{self, BoxStream},
    FutureExt, StreamExt,
};
use wsrpc::Service;

use crate::events::service::EventService;

pub struct SubscribeMonotonic {
    event_service: EventService,
}

impl Service for SubscribeMonotonic {
    type Req = SubscribeMonotonicRequest;
    type Resp = SubscribeMonotonicResponse;
    type Error = ();
    type Ctx = AppId;

    fn serve(&self, app_id: AppId, req: Self::Req) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        (async move { service.subscribe_monotonic(app_id, req).await })
            .map(|x| match x {
                Ok(stream) => stream.map(Ok).left_stream(),
                Err(_) => stream::once(futures::future::err(())).right_stream(),
            })
            .flatten_stream()
            .boxed()
    }
}

pub fn service(event_service: EventService) -> SubscribeMonotonic {
    SubscribeMonotonic { event_service }
}
