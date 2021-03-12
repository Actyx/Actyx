use actyxos_sdk::EventService;
use crypto::KeyStoreRef;
use warp::*;

mod http;
pub mod service;
mod ws;

pub fn routes<S: EventService + Clone + Send + Sync + 'static>(
    event_service: S,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    http::routes(event_service.clone(), key_store.clone()).or(ws::routes(event_service, key_store))
}
