#![doc(html_logo_url = "https://developer.actyx.com/img/logo.svg")]
#![doc(html_favicon_url = "https://developer.actyx.com/img/favicon.ico")]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::inconsistent_digit_grouping)]

#[doc(hidden)]
pub use ax_macros::*;

#[macro_use]
mod scalar;

mod app_manifest;
mod event;
mod offset;
mod scalars;
pub mod service;
mod tags;
mod timestamp;
pub mod types;

pub use app_manifest::AppManifest;
pub use event::{Event, EventKey, Metadata, Opaque, Payload};
pub use offset::{Offset, OffsetError, OffsetMap, OffsetOrMin};
pub use scalars::{AppId, NodeId, StreamId, StreamNr};
pub use tags::{Tag, TagSet};
pub use timestamp::{LamportTimestamp, Timestamp};

use derive_more::Display;
#[derive(Debug, Display, PartialEq, Eq)]
pub enum ParseError {
    #[display(fmt = "Empty string is not permissible for Tag")]
    EmptyTag,
    #[display(fmt = "Empty string is not permissible for AppId")]
    EmptyAppId,
    #[display(fmt = "Invalid AppId: '{}'", _0)]
    InvalidAppId(String),
}
impl std::error::Error for ParseError {}
