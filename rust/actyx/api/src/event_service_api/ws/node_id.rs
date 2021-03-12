use actyxos_sdk::{
    event_service::{EventService, NodeIdResponse},
    tagged::AppId,
};
use futures::{stream::BoxStream, FutureExt, StreamExt};
use wsrpc::Service;

pub struct NodeId<S: EventService + Send> {
    event_service: S,
}

impl<S: EventService + Send + Sync + 'static> Service for NodeId<S> {
    type Req = ();
    type Resp = NodeIdResponse;
    type Error = ();
    type Ctx = AppId;

    fn serve(&self, _app_id: AppId, _req: ()) -> BoxStream<'static, Result<Self::Resp, Self::Error>> {
        let service = self.event_service.clone();
        (async move { service.node_id().await.map_err(|_| ()) })
            .into_stream()
            .boxed()
    }
}

pub fn service<S: EventService>(event_service: S) -> NodeId<S> {
    NodeId { event_service }
}
