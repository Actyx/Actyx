use actyxos_sdk::{service::OffsetsResponse, AppId};
use futures::{
    stream::{BoxStream, StreamExt},
    FutureExt,
};
use wsrpc::Service;

use crate::events::service::EventService;

pub struct Offsets {
    event_service: EventService,
}

impl Service for Offsets {
    type Req = ();
    type Resp = OffsetsResponse;
    type Error = ();
    type Ctx = AppId;

    fn serve(&self, _app_id: AppId, _req: ()) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        (async move { service.offsets().await.map_err(|_| ()) })
            .into_stream()
            .boxed()
    }
}

pub fn service(event_service: EventService) -> Offsets {
    Offsets { event_service }
}
