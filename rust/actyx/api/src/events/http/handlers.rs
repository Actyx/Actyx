use super::ndjson;

use actyxos_sdk::{
    service::{PublishRequest, QueryRequest, SubscribeMonotonicRequest, SubscribeRequest},
    AppId,
};
use warp::*;

use crate::{
    events::service::{self, EventService},
    rejections::ApiError,
    util::{self, Result},
};

pub async fn offsets(_app_id: AppId, event_service: EventService) -> Result<impl Reply> {
    event_service
        .offsets()
        .await
        .map(|reply| reply::json(&reply))
        .map(|reply| reply::with_header(reply, http::header::CACHE_CONTROL, "no-cache"))
        .map_err(reject)
}

pub async fn publish(app_id: AppId, request: PublishRequest, event_service: EventService) -> Result<impl Reply> {
    event_service
        .publish(app_id, request)
        .await
        .map(|reply| reply::json(&reply))
        .map_err(reject)
}

pub async fn query(app_id: AppId, request: QueryRequest, event_service: EventService) -> Result<impl Reply> {
    event_service
        .query(app_id, request)
        .await
        .map(|events| ndjson::reply(ndjson::keep_alive().stream(events)))
        .map_err(reject)
}

pub async fn subscribe(app_id: AppId, request: SubscribeRequest, event_service: EventService) -> Result<impl Reply> {
    event_service
        .subscribe(app_id, request)
        .await
        .map(|events| ndjson::reply(ndjson::keep_alive().stream(events)))
        .map_err(reject)
}

pub async fn subscribe_monotonic(
    app_id: AppId,
    request: SubscribeMonotonicRequest,
    event_service: EventService,
) -> Result<impl Reply> {
    event_service
        .subscribe_monotonic(app_id, request)
        .await
        .map(|events| ndjson::reply(ndjson::keep_alive().stream(events)))
        .map_err(reject)
}

fn reject(err: anyhow::Error) -> Rejection {
    match err.downcast_ref::<service::Error>() {
        Some(service::Error::StoreReadError(_)) => reject::custom(ApiError::BadRequest { cause: err.to_string() }),
        _ => util::reject(err), // internal server errors
    }
}
