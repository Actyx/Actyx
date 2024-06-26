use super::ndjson;

use crate::{
    api::{events::service::EventService, rejections::ApiError, Result},
    runtime::features::FeatureError,
    swarm::event_store_ref,
};
use ax_types::{
    service::{PublishRequest, QueryRequest, SubscribeMonotonicRequest, SubscribeRequest},
    AppId,
};
use warp::{reply, Rejection, Reply};

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
    if let Some(e) = err.downcast_ref::<event_store_ref::Error>() {
        let cause = e.to_string();
        return match e {
            event_store_ref::Error::Aborted => warp::reject::custom(ApiError::Shutdown { cause }),
            event_store_ref::Error::Overload => warp::reject::custom(ApiError::Overloaded { cause }),
            event_store_ref::Error::InvalidUpperBounds => warp::reject::custom(ApiError::BadRequest { cause }),
            event_store_ref::Error::TagExprError(_) => warp::reject::custom(ApiError::BadRequest { cause }),
        };
    }
    let err = match err.downcast::<ApiError>() {
        Ok(e) => return warp::reject::custom(e),
        Err(e) => e,
    };
    match err.downcast::<FeatureError>() {
        Ok(e) => warp::reject::custom(ApiError::from(e)),
        Err(err) => {
            tracing::warn!("internal error: {:?}", err);
            crate::api::reject(err)
        }
    }
}
