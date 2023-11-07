use std::sync::Arc;

use maplit::btreemap;
use warp::*;
use wsrpc::Service;

use crate::api::api_util::{
    filters::{query_token, query_token_ws},
    NodeInfo,
};
use crate::api::events::service::EventService;

mod offsets;
mod publish;
mod query;
mod subscribe;
mod subscribe_monotonic;

pub(crate) fn routes(
    node_info: NodeInfo,
    event_service: EventService,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    // legacy support
    let token = query_token().or(query_token_ws()).unify();
    let auth = crate::api::api_util::filters::authenticate(node_info, token);
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