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
