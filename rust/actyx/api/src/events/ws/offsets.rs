use actyxos_sdk::{
    service::{EventService, OffsetsResponse},
    AppId,
};
use futures::{
    stream::{BoxStream, StreamExt},
    FutureExt,
};
use wsrpc::Service;

pub struct Offsets<S: EventService + Send> {
    event_service: S,
}

impl<S: EventService + Send + Sync + 'static> Service for Offsets<S> {
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

pub fn service<S: EventService>(event_service: S) -> Offsets<S> {
    Offsets { event_service }
}
