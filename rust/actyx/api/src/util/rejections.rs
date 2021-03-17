use actyx_util::serde_support::StringSerialized;
use actyxos_sdk::AppId;
use derive_more::Display;

#[derive(Debug, Clone)]
pub struct NotAcceptable {
    pub(crate) requested: String,
    pub(crate) supported: String,
}
impl warp::reject::Reject for NotAcceptable {}

#[derive(Debug, Display, Clone)]
pub enum TokenSource {
    #[display(fmt = "query parameter")]
    QueryParam,
    #[display(fmt = "header")]
    Header,
}

#[derive(Debug, Display, Clone)]
pub enum Unauthorized {
    MissingToken(TokenSource),
    TokenUnauthorized,
    UnsupportedAuthType(String),
    InvalidBearerToken(String),
    // AppUnauthorized(AppId),
    // AppUnauthenticated(AppId),
    InvalidSignature,
    // InvalidManifest,
}
impl warp::reject::Reject for Unauthorized {}

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
    InvalidManifest,

    #[display(fmt = "'{}' is not authorized. Provide a valid app license to the node.", app_id)]
    AppUnauthorized { app_id: AppId },

    #[display(fmt = "'{}' is not authenticated. Provided signature is invalid.", app_id)]
    AppUnauthenticated { app_id: AppId },

    #[display(
        fmt = "Authorization token is missing. Please provide a valid auth token {}.",
        source
    )]
    MissingAuthToken { source: TokenSource },

    #[display(fmt = "Authorization request header contains an unauthorized token.")]
    TokenUnauthorized,

    #[display(fmt = "Invalid token: '{}'. Please provide a valid Bearer token.", token)]
    TokenInvalid { token: String },

    #[display(
        fmt = "Unsupported Authorization header type '{}'. Please provide a Bearer token.",
        requested
    )]
    UnsupportedAuthType { requested: String },

    #[display(fmt = "Invalid request. {}", cause)]
    BadRequest { cause: String },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorResponse {
    pub code: String,
    pub message: StringSerialized<ApiError>,
}

impl From<ApiError> for ApiErrorResponse {
    fn from(e: ApiError) -> Self {
        let message = StringSerialized::from(e.clone());
        let code = match e {
            ApiError::AppUnauthenticated { .. } => "ERR_APP_UNAUTHENTICATED",
            ApiError::AppUnauthorized { .. } => "ERR_APP_UNAUTHORIZED",
            ApiError::BadRequest { .. } => "ERR_MALFORMED_REQUEST_SYNTAX",
            ApiError::InvalidManifest => "ERR_MANIFEST_INVALID",
            ApiError::MethodNotAllowed => "ERR_METHOD_NOT_ALLOWED",
            ApiError::MissingAuthToken { .. } => "ERR_EMPTY_AUTH_HEADER",
            ApiError::NotAcceptable { .. } => "ERR_NOT_ACCEPTABLE",
            ApiError::TokenInvalid { .. } => "ERR_TOKEN_INVALID",
            ApiError::TokenUnauthorized => "ERR_TOKEN_UNAUTHORIZED",
            ApiError::UnsupportedAuthType { .. } => "ERR_WRONG_AUTH_TYPE",
        }
        .to_string();
        ApiErrorResponse { code, message }
    }
}
