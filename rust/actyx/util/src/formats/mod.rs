use derive_more::{AsRef, Display, From, Into};
use serde::Deserialize;

pub mod admin_protocol;
pub mod errors;
pub mod logs;

pub use admin_protocol::*;
pub use errors::*;
pub use logs::*;

pub const ACTYXOS_ID: &str = "com.actyx.os";

#[derive(Deserialize, PartialEq, Clone, Debug, From, Into, AsRef, Display)]
pub struct NodeName(pub String);
