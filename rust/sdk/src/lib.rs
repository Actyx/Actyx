#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://developer.actyx.com/img/logo.svg")]
#![doc(html_favicon_url = "https://developer.actyx.com/img/favicon.ico")]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::inconsistent_digit_grouping)]

mod client;
pub mod files;

pub use client::{Ax, AxOpts, Publish, Query, Subscribe, SubscribeMonotonic};

pub use ax_aql as aql;
pub use ax_types as types;

pub use url::Url;
