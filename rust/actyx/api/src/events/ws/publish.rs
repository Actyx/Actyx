use actyx_sdk::service::{PublishRequest, PublishResponse};
use futures::{stream::BoxStream, FutureExt, StreamExt};
use wsrpc::Service;

use crate::{events::service::EventService, BearerToken};

pub struct Publish {
    event_service: EventService,
}

impl Service for Publish {
    type Req = PublishRequest;
    type Resp = PublishResponse;
    type Error = String;
    type Ctx = BearerToken;

    fn serve(&self, bearer_token: BearerToken, req: Self::Req) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        let BearerToken {
            app_id, app_version, ..
        } = bearer_token;
        (async move {
            service
                .publish(app_id, app_version, req)
                .await
                .map_err(|e| e.to_string())
        })
        .into_stream()
        .boxed()
    }
}

pub fn service(event_service: EventService) -> Publish {
    Publish { event_service }
}
