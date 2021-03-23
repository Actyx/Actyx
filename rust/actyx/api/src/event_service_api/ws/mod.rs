use std::sync::Arc;

use actyxos_sdk::{service::EventService, NodeId};
use crypto::KeyStoreRef;
use maplit::btreemap;
use warp::*;
use wsrpc::Service;

use crate::util::filters::query_token;

mod node_id;
mod offsets;
mod publish;
mod query;
mod subscribe;
mod subscribe_monotonic;

pub(crate) fn routes<S: EventService + Clone + Send + Sync + 'static>(
    node_id: NodeId,
    event_service: S,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    // TODO replace with crate::util::filters::authenticate
    let auth = super::auth_mock::authenticate(query_token(), key_store, node_id.into());
    let services = Arc::new(btreemap! {
      "node_id"             => node_id::service(event_service.clone()).boxed(),
      "offsets"             => offsets::service(event_service.clone()).boxed(),
      "query"               => query::service(event_service.clone()).boxed(),
      "subscribe"           => subscribe::service(event_service.clone()).boxed(),
      "subscribe_monotonic" => subscribe_monotonic::service(event_service.clone()).boxed(),
      "publish"             => publish::service(event_service).boxed(),
    });

    warp::path::end()
        .and(warp::ws())
        .and(warp::any().map(move || services.clone()))
        .and(auth)
        .and_then(wsrpc::serve)
}
