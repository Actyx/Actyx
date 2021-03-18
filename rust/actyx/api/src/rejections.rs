use actyx_util::serde_support::StringSerialized;
use actyxos_sdk::AppId;
use derive_more::Display;
use tracing::*;
use warp::{http::StatusCode, *};

#[derive(Debug, Display, Clone)]
pub enum TokenSource {
    #[display(fmt = "query parameter")]
    QueryParam,
    #[display(fmt = "header")]
    Header,
}

#[derive(Debug, Display, Clone)]
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

    #[display(fmt = "Property <manifest property> is either missing or has an invalid value.")]
    #[allow(dead_code)]
    InvalidManifest,

    #[display(fmt = "Not authorized. Provide a valid app license to the node.")]
    #[allow(dead_code)]
    AppUnauthorized,

    #[display(fmt = "'{}' is not authenticated. Provided signature is invalid.", app_id)]
    #[allow(dead_code)]
    AppUnauthenticated { app_id: AppId },

    #[display(
        fmt = "Authorization token is missing. Please provide a valid auth token {}.",
        source
    )]
    MissingAuthToken { source: TokenSource },

    #[display(fmt = "Authorization request header contains an unauthorized token.")]
    TokenUnauthorized,

    #[display(fmt = "Invalid token: '{}'. {} Please provide a valid Bearer token.", token, msg)]
    TokenInvalid { token: String, msg: String },

    #[display(
        fmt = "Unsupported Authorization header type '{}'. Please provide a Bearer token.",
        requested
    )]
    UnsupportedAuthType { requested: String },

    #[display(fmt = "Invalid request. {}", cause)]
    BadRequest { cause: String },

    #[display(fmt = "Internal server error.")]
    Internal,
}
impl warp::reject::Reject for ApiError {}

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
        let message: StringSerialized<_> = e.clone().into();
        let (status, code) = match e {
            ApiError::AppUnauthenticated { .. } => (StatusCode::UNAUTHORIZED, "ERR_APP_UNAUTHENTICATED"),
            ApiError::AppUnauthorized { .. } => (StatusCode::UNAUTHORIZED, "ERR_APP_UNAUTHORIZED"),
            ApiError::BadRequest { .. } => (StatusCode::BAD_REQUEST, "ERR_MALFORMED_REQUEST_SYNTAX"),
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "ERR_INTERNAL"),
            ApiError::InvalidManifest => (StatusCode::BAD_REQUEST, "ERR_MANIFEST_INVALID"),
            ApiError::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "ERR_METHOD_NOT_ALLOWED"),
            ApiError::MissingAuthToken { .. } => (StatusCode::UNAUTHORIZED, "ERR_EMPTY_AUTH_HEADER"),
            ApiError::NotAcceptable { .. } => (StatusCode::NOT_ACCEPTABLE, "ERR_NOT_ACCEPTABLE"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "ERR_NOT_FOUND"),
            ApiError::TokenInvalid { .. } => (StatusCode::BAD_REQUEST, "ERR_TOKEN_INVALID"),
            ApiError::TokenUnauthorized => (StatusCode::UNAUTHORIZED, "ERR_TOKEN_UNAUTHORIZED"),
            ApiError::UnsupportedAuthType { .. } => (StatusCode::UNAUTHORIZED, "ERR_WRONG_AUTH_TYPE"),
        };
        ApiErrorResponse {
            code: code.to_string(),
            status,
            message,
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
    } else if let Some(reject::MethodNotAllowed { .. }) = r.find() {
        ApiError::MethodNotAllowed
    } else if let Some(e) = r.find::<ApiError>() {
        e.to_owned()
    } else if let Some(e) = r.find::<filters::body::BodyDeserializeError>() {
        use std::error::Error;
        ApiError::BadRequest {
            cause: e.source().map_or("".to_string(), |e| e.to_string()),
        }
    } else {
        warn!("unhandled rejection: {:?}", r);
        ApiError::Internal
    };
    let err_resp: ApiErrorResponse = api_err.into();
    let json = warp::reply::json(&err_resp);
    Ok(warp::reply::with_status(json, err_resp.status))
}
