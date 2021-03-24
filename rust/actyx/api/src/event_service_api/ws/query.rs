use actyxos_sdk::{
    service::{EventService, QueryRequest, QueryResponse},
    AppId,
};
use futures::{
    stream::{self, BoxStream},
    FutureExt, StreamExt,
};
use wsrpc::Service;

pub struct Query<S: EventService + Send> {
    event_service: S,
}

impl<S: EventService + Send + Sync + 'static> Service for Query<S> {
    type Req = QueryRequest;
    type Resp = QueryResponse;
    type Error = ();
    type Ctx = AppId;

    fn serve(&self, _app_id: AppId, req: Self::Req) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        (async move { service.query(req).await })
            .map(|x| match x {
                Ok(stream) => stream.map(Ok).left_stream(),
                Err(_) => stream::once(futures::future::err(())).right_stream(),
            })
            .flatten_stream()
            .boxed()
    }
}

pub fn service<S: EventService>(event_service: S) -> Query<S> {
    Query { event_service }
}
