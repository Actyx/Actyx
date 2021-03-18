use actyx_util::serde_support::StringSerialized;
use actyxos_sdk::AppId;
use derive_more::Display;
use warp::http::StatusCode;

#[derive(Debug, Display, Clone)]
pub enum TokenSource {
    #[display(fmt = "query parameter")]
    QueryParam,
    #[display(fmt = "header")]
    Header,
}

#[derive(Debug, Display, Clone)]
pub enum ApiError {
    #[display(fmt = "Method not supported.")]
    MethodNotAllowed,

    #[display(
        fmt = "The requested resource is only capable of generating content of type '{}' but '{}' was requested.",
        supported,
        requested
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
            ApiError::BadRequest { .. } => (StatusCode::BAD_REQUEST, "ERR_MALFORMED_REQUEST_SYNTAX"),
            ApiError::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "ERR_METHOD_NOT_ALLOWED"),
            ApiError::MissingAuthToken { .. } => (StatusCode::UNAUTHORIZED, "ERR_EMPTY_AUTH_HEADER"),
            ApiError::NotAcceptable { .. } => (StatusCode::NOT_ACCEPTABLE, "ERR_NOT_ACCEPTABLE"),
            ApiError::TokenInvalid { .. } => (StatusCode::BAD_REQUEST, "ERR_TOKEN_INVALID"),
            ApiError::TokenUnauthorized => (StatusCode::UNAUTHORIZED, "ERR_TOKEN_UNAUTHORIZED"),
            ApiError::UnsupportedAuthType { .. } => (StatusCode::UNAUTHORIZED, "ERR_WRONG_AUTH_TYPE"),
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "ERR_INTERNAL"),
            ApiError::AppUnauthenticated { .. } => (StatusCode::UNAUTHORIZED, "ERR_APP_UNAUTHENTICATED"),
            ApiError::AppUnauthorized { .. } => (StatusCode::UNAUTHORIZED, "ERR_APP_UNAUTHORIZED"),
            ApiError::InvalidManifest => (StatusCode::BAD_REQUEST, "ERR_MANIFEST_INVALID"),
        };
        ApiErrorResponse {
            code: code.to_string(),
            status,
            message,
        }
    }
}
