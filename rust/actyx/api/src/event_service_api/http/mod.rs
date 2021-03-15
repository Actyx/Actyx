mod filters;
mod handlers;
mod ndjson;

use actyxos_sdk::service::EventService;
use crypto::KeyStoreRef;
use warp::Filter;

pub(crate) fn routes<S: EventService + Clone + Send + Sync + 'static>(
    event_service: S,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    filters::node_id(event_service.clone(), key_store.clone())
        .or(filters::offsets(event_service.clone(), key_store.clone()))
        .or(filters::publish(event_service.clone(), key_store.clone()))
        .or(filters::query(event_service.clone(), key_store.clone()))
        .or(filters::subscribe(event_service.clone(), key_store.clone()))
        .or(filters::subscribe_monotonic(event_service, key_store))
}
