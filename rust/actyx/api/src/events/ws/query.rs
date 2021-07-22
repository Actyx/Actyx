use actyx_sdk::{
    service::{QueryRequest, QueryResponse},
    AppId,
};
use futures::{
    stream::{self, BoxStream},
    FutureExt, StreamExt,
};
use wsrpc::Service;

use crate::events::service::EventService;

pub struct Query {
    event_service: EventService,
}

impl Service for Query {
    type Req = QueryRequest;
    type Resp = QueryResponse;
    type Error = String;
    type Ctx = AppId;

    fn serve(&self, app_id: AppId, req: Self::Req) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        (async move { service.query(app_id, req).await })
            .map(|x| match x {
                Ok(stream) => stream.map(Ok).left_stream(),
                Err(e) => stream::once(futures::future::err(e.to_string())).right_stream(),
            })
            .flatten_stream()
            .boxed()
    }
}

pub fn service(event_service: EventService) -> Query {
    Query { event_service }
}
