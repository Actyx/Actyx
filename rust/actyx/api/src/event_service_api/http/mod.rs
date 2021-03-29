mod filters;
mod handlers;
mod ndjson;

use actyxos_sdk::{service::EventService, NodeId};
use crypto::KeyStoreRef;
use warp::Filter;

use crate::util::filters::header_token;

pub(crate) fn routes<S: EventService + Clone + Send + Sync + 'static>(
    node_id: NodeId,
    event_service: S,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    // TODO replace with crate::util::filters::authenticate
    let auth = super::auth_mock::authenticate(header_token(), key_store, node_id.into());

    filters::node_id(event_service.clone(), auth.clone())
        .or(filters::offsets(event_service.clone(), auth.clone()))
        .or(filters::publish(event_service.clone(), auth.clone()))
        .or(filters::query(event_service.clone(), auth.clone()))
        .or(filters::subscribe(event_service.clone(), auth.clone()))
        .or(filters::subscribe_monotonic(event_service, auth))
}
