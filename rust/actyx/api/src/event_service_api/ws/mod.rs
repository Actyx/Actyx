use std::str::FromStr;
use std::sync::Arc;

use actyxos_sdk::{service::EventService, AppId};
use crypto::KeyStoreRef;
use maplit::btreemap;
use warp::*;
use wsrpc::Service;

mod node_id;
mod offsets;
mod publish;
mod query;
mod subscribe;
mod subscribe_monotonic;

#[derive(Debug)]
struct AuthErr;
impl reject::Reject for AuthErr {}
pub fn authenticate(_key_store: KeyStoreRef) -> impl Filter<Extract = (AppId,), Error = Rejection> + Clone {
    any().and_then(move || async move { AppId::from_str("placeholder").map_err(|_| reject::custom(AuthErr)) })
}

pub(crate) fn routes<S: EventService + Clone + Send + Sync + 'static>(
    event_service: S,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
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
        .and(authenticate(key_store))
        .and_then(wsrpc::serve)
}
