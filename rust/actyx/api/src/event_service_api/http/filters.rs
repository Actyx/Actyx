use actyxos_sdk::{service::EventService, AppId};
use warp::filters::*;
use warp::*;

use crate::event_service_api::http::handlers;

pub fn with_service(
    event_service: impl EventService + Send,
) -> impl Filter<Extract = (impl EventService,), Error = std::convert::Infallible> + Clone {
    any().map(move || event_service.clone())
}

fn accept_json() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    crate::util::filters::accept("application/json")
}

pub fn node_id(
    event_service: impl EventService + Send + Sync + 'static,
    auth: impl Filter<Extract = (AppId,), Error = Rejection> + Clone,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    auth.and(path!("node_id"))
        .and(get())
        .and(accept_json())
        .and(with_service(event_service))
        .and_then(handlers::node_id)
}

pub fn offsets(
    event_service: impl EventService + Send + Sync + 'static,
    auth: impl Filter<Extract = (AppId,), Error = Rejection> + Clone,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    auth.and(path!("offsets"))
        .and(get())
        .and(accept_json())
        .and(with_service(event_service))
        .and_then(handlers::offsets)
}

pub fn publish(
    event_service: impl EventService + Send + Sync + 'static,
    auth: impl Filter<Extract = (AppId,), Error = Rejection> + Clone,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    auth.and(path!("publish"))
        .and(post())
        .and(accept_json())
        .and(body::json())
        .and(with_service(event_service))
        .and_then(handlers::publish)
}

pub fn query(
    event_service: impl EventService + Send + Sync + 'static,
    auth: impl Filter<Extract = (AppId,), Error = Rejection> + Clone,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    auth.and(path!("query"))
        .and(post())
        .and(accept_json())
        .and(body::json())
        .and(with_service(event_service))
        .and_then(handlers::query)
}

pub fn subscribe(
    event_service: impl EventService + Send + Sync + 'static,
    auth: impl Filter<Extract = (AppId,), Error = Rejection> + Clone,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    auth.and(path!("subscribe"))
        .and(post())
        .and(accept_json())
        .and(body::json())
        .and(with_service(event_service))
        .and_then(handlers::subscribe)
}

pub fn subscribe_monotonic(
    event_service: impl EventService + Send + Sync + 'static,
    auth: impl Filter<Extract = (AppId,), Error = Rejection> + Clone,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    auth.and(path!("subscribe_monotonic"))
        .and(post())
        .and(accept_json())
        .and(body::json())
        .and(with_service(event_service))
        .and_then(handlers::subscribe_monotonic)
}
