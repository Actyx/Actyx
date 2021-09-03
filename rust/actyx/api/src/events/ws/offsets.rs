use actyx_sdk::service::OffsetsResponse;
use futures::{
    stream::{BoxStream, StreamExt},
    FutureExt,
};
use wsrpc::Service;

use crate::{events::service::EventService, BearerToken};

pub struct Offsets {
    event_service: EventService,
}

impl Service for Offsets {
    type Req = ();
    type Resp = OffsetsResponse;
    type Error = String;
    type Ctx = BearerToken;

    fn serve(&self, _bearer_token: BearerToken, _req: ()) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        (async move { service.offsets().await.map_err(|e| e.to_string()) })
            .into_stream()
            .boxed()
    }
}

pub fn service(event_service: EventService) -> Offsets {
    Offsets { event_service }
}
