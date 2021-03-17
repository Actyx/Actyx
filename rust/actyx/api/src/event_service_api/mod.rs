use actyxos_sdk::{service::EventService, NodeId};
use crypto::KeyStoreRef;
use warp::*;

mod auth_mock;
mod http;
pub mod service;
mod ws;

pub fn routes<S: EventService + Clone + Send + Sync + 'static>(
    node_id: NodeId,
    event_service: S,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    http::routes(node_id, event_service.clone(), key_store.clone()).or(ws::routes(node_id, event_service, key_store))
}
