use actyxos_sdk::service::EventService;
use crypto::KeyStoreRef;
use warp::*;

mod http;
pub mod service;
mod ws;

pub fn routes<S: EventService + Clone + Send + Sync + 'static>(
    event_service: S,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    http::routes(event_service.clone(), key_store.clone()).or(ws::routes(event_service, key_store))
}
