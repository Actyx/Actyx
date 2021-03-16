use std::str::FromStr;

use actyxos_sdk::{service::EventService, AppId};
use crypto::KeyStoreRef;
use warp::*;

use crate::event_service_api::http::handlers;

#[derive(Debug)]
struct AuthErr;
impl reject::Reject for AuthErr {}
pub fn authenticate(_key_store: KeyStoreRef) -> impl Filter<Extract = (AppId,), Error = Rejection> + Clone {
    any().and_then(move || async move { AppId::from_str("placeholder").map_err(|_| reject::custom(AuthErr)) })
}

pub fn with_service(
    event_service: impl EventService + Send,
) -> impl Filter<Extract = (impl EventService,), Error = std::convert::Infallible> + Clone {
    any().map(move || event_service.clone())
}

fn accept_json() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    crate::util::filter::accept("application/json")
}

pub fn node_id(
    event_service: impl EventService + Send + Sync + 'static,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path!("node_id")
        .and(get())
        .and(accept_json())
        .and(with_service(event_service))
        .and(authenticate(key_store))
        .and_then(handlers::node_id)
}

pub fn offsets(
    event_service: impl EventService + Send + Sync + 'static,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path!("offsets")
        .and(get())
        .and(accept_json())
        .and(with_service(event_service))
        .and(authenticate(key_store))
        .and_then(handlers::offsets)
}

pub fn publish(
    event_service: impl EventService + Send + Sync + 'static,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path!("publish")
        .and(post())
        .and(accept_json())
        .and(body::json())
        .and(with_service(event_service))
        .and(authenticate(key_store))
        .and_then(handlers::publish)
}

pub fn query(
    event_service: impl EventService + Send + Sync + 'static,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path!("query")
        .and(post())
        .and(accept_json())
        .and(body::json())
        .and(with_service(event_service))
        .and(authenticate(key_store))
        .and_then(handlers::query)
}

pub fn subscribe(
    event_service: impl EventService + Send + Sync + 'static,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path!("subscribe")
        .and(post())
        .and(accept_json())
        .and(body::json())
        .and(with_service(event_service))
        .and(authenticate(key_store))
        .and_then(handlers::subscribe)
}

pub fn subscribe_monotonic(
    event_service: impl EventService + Send + Sync + 'static,
    key_store: KeyStoreRef,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    path!("subscribe_monotonic")
        .and(post())
        .and(accept_json())
        .and(body::json())
        .and(with_service(event_service))
        .and(authenticate(key_store))
        .and_then(handlers::subscribe_monotonic)
}
