use actyx_sdk::AppId;
use actyx_util::serde_support::StringSerialized;
use derive_more::Display;
use runtime::features::FeatureError;
use tracing::*;
use warp::{http::StatusCode, *};

#[derive(Debug, Display, Clone, PartialEq)]
pub enum UnauthorizedReason {
    #[display(fmt = "no license found")]
    NoLicense,
    #[display(fmt = "invalid license key format")]
    MalformedLicense,
    #[display(fmt = "invalid signature")]
    InvalidSignature,
    #[display(fmt = "wrong license subject")]
    WrongSubject,
    #[display(fmt = "license expired")]
    Expired,
}

#[derive(Debug, Display, Clone, PartialEq)]
pub enum ApiError {
    #[display(fmt = "The requested resource could not be found.")]
    NotFound,

    #[display(fmt = "Method not supported.")]
    MethodNotAllowed,

    #[display(
        fmt = "Content with type '{}' was requested but the resource is only capable of generating content of the following type(s): {}.",
        requested,
        supported
    )]
    NotAcceptable { supported: String, requested: String },

    #[display(fmt = "Invalid manifest. {}", msg)]
    InvalidManifest { msg: String },

    #[display(
        fmt = "App '{}' is not authorized: {}. Provide a valid app license in the node settings.",
        app_id,
        reason
    )]
    AppUnauthorized { app_id: AppId, reason: UnauthorizedReason },

    #[display(fmt = "Node is not licensed: {}. Please correct this in the settings.", reason)]
    NodeUnauthorized { reason: UnauthorizedReason },

    #[display(fmt = "\"Authorization\" header is missing.")]
    MissingAuthorizationHeader,

    #[display(fmt = "\"token\" parameter is missing.")]
    MissingTokenParameter,

    #[display(fmt = "Unauthorized token.")]
    TokenUnauthorized,

    #[display(fmt = "Expired token.")]
    TokenExpired,

    #[display(fmt = "Invalid token: '{}'. {} Please provide a valid bearer token.", token, msg)]
    TokenInvalid { token: String, msg: String },

    #[display(fmt = "{}.", msg)]
    UnsupportedMediaType { msg: String },

    #[display(
        fmt = "Unsupported authentication type '{}'. Only \"Bearer\" is supported.",
        requested
    )]
    UnsupportedAuthType { requested: String },

    #[display(fmt = "Invalid request. {}", cause)]
    BadRequest { cause: String },

    #[display(fmt = "Feature `{}` is not supported on endpoint `{}`.", features, endpoint)]
    UnsupportedFeature { features: String, endpoint: String },

    #[display(fmt = "Internal server error.")]
    Internal,

    #[display(fmt = "Service overloaded. {}", cause)]
    Overloaded { cause: String },

    #[display(fmt = "Service shutting down. {}", cause)]
    Shutdown { cause: String },
}
impl warp::reject::Reject for ApiError {}
impl std::error::Error for ApiError {}

impl From<FeatureError> for ApiError {
    fn from(e: FeatureError) -> Self {
        match e {
            FeatureError::Alpha(_) => ApiError::BadRequest { cause: e.to_string() },
            FeatureError::Beta(_) => ApiError::BadRequest { cause: e.to_string() },
            FeatureError::Unsupported { features, endpoint } => ApiError::UnsupportedFeature { features, endpoint },
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorResponse {
    #[serde(skip)]
    pub status: StatusCode,
    pub code: String,
    pub message: StringSerialized<ApiError>,
}
impl From<ApiError> for ApiErrorResponse {
    fn from(e: ApiError) -> Self {
        let (status, code) = match &e {
            ApiError::AppUnauthorized { .. } => (StatusCode::UNAUTHORIZED, "ERR_APP_UNAUTHORIZED"),
            ApiError::NodeUnauthorized { .. } => (StatusCode::UNAUTHORIZED, "ERR_NODE_UNAUTHORIZED"),
            ApiError::BadRequest { .. } => (StatusCode::BAD_REQUEST, "ERR_BAD_REQUEST"),
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "ERR_INTERNAL"),
            ApiError::InvalidManifest { .. } => (StatusCode::BAD_REQUEST, "ERR_MANIFEST_INVALID"),
            ApiError::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "ERR_METHOD_NOT_ALLOWED"),
            ApiError::MissingAuthorizationHeader => (StatusCode::UNAUTHORIZED, "ERR_MISSING_AUTH_HEADER"),
            ApiError::MissingTokenParameter => (StatusCode::UNAUTHORIZED, "ERR_MISSING_TOKEN_PARAM"),
            ApiError::NotAcceptable { .. } => (StatusCode::NOT_ACCEPTABLE, "ERR_NOT_ACCEPTABLE"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "ERR_NOT_FOUND"),
            ApiError::Overloaded { .. } => (StatusCode::SERVICE_UNAVAILABLE, "ERR_SERVICE_OVERLOADED"),
            ApiError::Shutdown { .. } => (StatusCode::SERVICE_UNAVAILABLE, "ERR_SHUTTING_DOWN"),
            ApiError::TokenExpired => (StatusCode::UNAUTHORIZED, "ERR_TOKEN_EXPIRED"),
            ApiError::TokenInvalid { .. } => (StatusCode::BAD_REQUEST, "ERR_TOKEN_INVALID"),
            ApiError::TokenUnauthorized => (StatusCode::UNAUTHORIZED, "ERR_TOKEN_UNAUTHORIZED"),
            ApiError::UnsupportedAuthType { .. } => (StatusCode::UNAUTHORIZED, "ERR_UNSUPPORTED_AUTH_TYPE"),
            ApiError::UnsupportedFeature { .. } => (StatusCode::IM_A_TEAPOT, "ERR_UNSUPPORTED_FEATURE"),
            ApiError::UnsupportedMediaType { .. } => (StatusCode::UNSUPPORTED_MEDIA_TYPE, "ERR_UNSUPPORTED_MEDIA_TYPE"),
        };
        ApiErrorResponse {
            code: code.to_string(),
            status,
            message: e.into(),
        }
    }
}

/// Internal rejection used for testing purposes
#[derive(Debug)]
pub(crate) struct Crash;
impl reject::Reject for Crash {}

pub fn handle_rejection(r: Rejection) -> Result<impl Reply, Rejection> {
    let api_err = if r.is_not_found() {
        ApiError::NotFound
    } else if let Some(umt) = r.find::<reject::UnsupportedMediaType>() {
        ApiError::UnsupportedMediaType { msg: umt.to_string() }
    } else if let Some(e) = r.find::<ApiError>() {
        if let ApiError::AppUnauthorized { app_id, reason } = e {
            info!(target: "AUTH", "Unauthorized app {}. {}.", app_id, reason)
        }
        e.to_owned()
    } else if let Some(e) = r.find::<filters::body::BodyDeserializeError>() {
        use std::error::Error;
        ApiError::BadRequest {
            cause: e.source().map_or("unknown".to_string(), |e| e.to_string()),
        }
    } else if r.find::<reject::MethodNotAllowed>().is_some() {
        ApiError::MethodNotAllowed
    } else {
        warn!("unhandled rejection: {:?}", r);
        ApiError::Internal
    };

    let err_resp: ApiErrorResponse = api_err.into();
    let json = warp::reply::json(&err_resp);
    Ok(warp::reply::with_status(json, err_resp.status))
}
