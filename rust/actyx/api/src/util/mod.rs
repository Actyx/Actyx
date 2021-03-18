pub mod filters;
pub mod hyper_serve;

use derive_more::Display;

#[derive(Debug, Display, serde::Deserialize)]
pub struct Token(pub(crate) String);

#[derive(Debug, serde::Deserialize)]
pub struct Params {
    pub(crate) token: Token,
}
