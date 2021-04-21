use std::sync::Arc;

use actyxos_sdk::service::EventService;
use maplit::btreemap;
use warp::*;
use wsrpc::Service;

use crate::util::{filters::query_token, AuthArgs};

mod node_id;
mod offsets;
mod publish;
mod query;
mod subscribe;
mod subscribe_monotonic;

pub(crate) fn routes<S: EventService + Clone + Send + Sync + 'static>(
    auth_args: AuthArgs,
    event_service: S,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let auth = crate::util::filters::authenticate(auth_args, query_token());
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
