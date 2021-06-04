use actyxos_sdk::{
    service::{SubscribeRequest, SubscribeResponse},
    AppId,
};
use futures::{
    stream::{self, BoxStream},
    FutureExt, StreamExt,
};
use wsrpc::Service;

use crate::events::service::EventService;

pub struct Subscribe {
    event_service: EventService,
}

impl Service for Subscribe {
    type Req = SubscribeRequest;
    type Resp = SubscribeResponse;
    type Error = ();
    type Ctx = AppId;

    fn serve(&self, app_id: AppId, req: Self::Req) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        async move {
            service
                .subscribe(app_id, req)
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

pub fn service(event_service: EventService) -> Subscribe {
    Subscribe { event_service }
}
