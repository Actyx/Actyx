use crate::api::{
    events::{http::handlers, service::EventService},
    filters::{accept_json, accept_ndjson, authenticate, header_or_query_token},
    NodeInfo,
};
use actyx_sdk::AppId;
use warp::{any, body, get, path, post, Filter, Rejection, Reply};

pub fn with_service(
    event_service: EventService,
) -> impl Filter<Extract = (EventService,), Error = std::convert::Infallible> + Clone {
    any().map(move || event_service.clone())
}

fn authorize(node_info: NodeInfo) -> impl Filter<Extract = (AppId,), Error = Rejection> + Clone {
    authenticate(node_info, header_or_query_token())
}

pub fn offsets(
    node_info: NodeInfo,
    event_service: EventService,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    path("offsets")
        .and(path::end())
        .and(get())
        .and(authorize(node_info))
        .and(accept_json())
        .and(with_service(event_service))
        .and_then(handlers::offsets)
}

pub fn publish(
    node_info: NodeInfo,
    event_service: EventService,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    path("publish")
        .and(path::end())
        .and(post())
        .and(authorize(node_info))
        .and(accept_json())
        .and(body::json())
        .and(with_service(event_service))
        .and_then(handlers::publish)
}

pub fn query(
    node_info: NodeInfo,
    event_service: EventService,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    path("query")
        .and(path::end())
        .and(post())
        .and(authorize(node_info))
        .and(accept_ndjson())
        .and(body::json())
        .and(with_service(event_service))
        .and_then(handlers::query)
}

pub fn subscribe(
    node_info: NodeInfo,
    event_service: EventService,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    path("subscribe")
        .and(path::end())
        .and(post())
        .and(authorize(node_info))
        .and(accept_ndjson())
        .and(body::json())
        .and(with_service(event_service))
        .and_then(handlers::subscribe)
}

pub fn subscribe_monotonic(
    node_info: NodeInfo,
    event_service: EventService,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    path("subscribe_monotonic")
        .and(path::end())
        .and(post())
        .and(authorize(node_info))
        .and(accept_ndjson())
        .and(body::json())
        .and(with_service(event_service))
        .and_then(handlers::subscribe_monotonic)
}
