use std::sync::Arc;

use maplit::btreemap;
use warp::*;
use wsrpc::Service;

use crate::events::service::EventService;
use crate::util::{filters::query_token, NodeInfo};

mod offsets;
mod publish;
mod query;
mod subscribe;
mod subscribe_monotonic;

pub(crate) fn routes(
    auth_args: NodeInfo,
    event_service: EventService,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let auth = crate::util::filters::authenticate(auth_args, query_token());
    let services = Arc::new(btreemap! {
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
