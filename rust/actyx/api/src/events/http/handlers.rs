use super::ndjson;

use actyx_sdk::service::{PublishRequest, QueryRequest, SubscribeMonotonicRequest, SubscribeRequest};
use warp::*;

use crate::{
    events::service::EventService,
    rejections::ApiError,
    util::{self, Result},
    BearerToken,
};
use runtime::features::FeatureError;
use swarm::event_store_ref;

pub async fn offsets(_bearer_token: BearerToken, event_service: EventService) -> Result<impl Reply> {
    event_service
        .offsets()
        .await
        .map(|reply| reply::json(&reply))
        .map(|reply| reply::with_header(reply, http::header::CACHE_CONTROL, "no-cache"))
        .map_err(reject)
}

pub async fn publish(
    bearer_token: BearerToken,
    request: PublishRequest,
    event_service: EventService,
) -> Result<impl Reply> {
    let BearerToken {
        app_id, app_version, ..
    } = bearer_token;
    event_service
        .publish(app_id, app_version, request)
        .await
        .map(|reply| reply::json(&reply))
        .map_err(reject)
}

pub async fn query(
    bearer_token: BearerToken,
    request: QueryRequest,
    event_service: EventService,
) -> Result<impl Reply> {
    let BearerToken {
        app_id, app_version, ..
    } = bearer_token;
    event_service
        .query(app_id, app_version, request)
        .await
        .map(|events| ndjson::reply(ndjson::keep_alive().stream(events)))
        .map_err(reject)
}

pub async fn subscribe(
    bearer_token: BearerToken,
    request: SubscribeRequest,
    event_service: EventService,
) -> Result<impl Reply> {
    let BearerToken {
        app_id, app_version, ..
    } = bearer_token;
    event_service
        .subscribe(app_id, app_version, request)
        .await
        .map(|events| ndjson::reply(ndjson::keep_alive().stream(events)))
        .map_err(reject)
}

pub async fn subscribe_monotonic(
    bearer_token: BearerToken,
    request: SubscribeMonotonicRequest,
    event_service: EventService,
) -> Result<impl Reply> {
    let BearerToken {
        app_id, app_version, ..
    } = bearer_token;
    event_service
        .subscribe_monotonic(app_id, app_version, request)
        .await
        .map(|events| ndjson::reply(ndjson::keep_alive().stream(events)))
        .map_err(reject)
}

fn reject(err: anyhow::Error) -> Rejection {
    if let Some(e) = err.downcast_ref::<event_store_ref::Error>() {
        let cause = e.to_string();
        match e {
            event_store_ref::Error::Aborted => reject::custom(ApiError::Shutdown { cause }),
            event_store_ref::Error::Overload => reject::custom(ApiError::Overloaded { cause }),
            event_store_ref::Error::InvalidUpperBounds => reject::custom(ApiError::BadRequest { cause }),
            event_store_ref::Error::TagExprError(_) => reject::custom(ApiError::BadRequest { cause }),
        }
    } else {
        match err.downcast::<FeatureError>() {
            Ok(e) => reject::custom(ApiError::from(e)),
            Err(err) => {
                tracing::warn!("internal error: {:?}", err);
                util::reject(err)
            }
        }
    }
}
