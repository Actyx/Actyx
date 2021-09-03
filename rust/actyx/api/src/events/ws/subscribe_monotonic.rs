use actyx_sdk::service::{SubscribeMonotonicRequest, SubscribeMonotonicResponse};
use futures::{
    stream::{self, BoxStream},
    FutureExt, StreamExt,
};
use wsrpc::Service;

use crate::{events::service::EventService, BearerToken};

pub struct SubscribeMonotonic {
    event_service: EventService,
}

impl Service for SubscribeMonotonic {
    type Req = SubscribeMonotonicRequest;
    type Resp = SubscribeMonotonicResponse;
    type Error = String;
    type Ctx = BearerToken;

    fn serve(&self, bearer_token: BearerToken, req: Self::Req) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        let BearerToken {
            app_id, app_version, ..
        } = bearer_token;
        (async move { service.subscribe_monotonic(app_id, app_version, req).await })
            .map(|x| match x {
                Ok(stream) => stream.map(Ok).left_stream(),
                Err(e) => stream::once(futures::future::err(e.to_string())).right_stream(),
            })
            .flatten_stream()
            .boxed()
    }
}

pub fn service(event_service: EventService) -> SubscribeMonotonic {
    SubscribeMonotonic { event_service }
}
