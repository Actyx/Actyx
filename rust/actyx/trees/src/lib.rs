#![deny(clippy::future_not_send)]
#[cfg(any(test, feature = "arb"))]
mod arb;
pub mod axtrees;
mod dnf;
mod header;
pub mod offsetmap_or_default;
pub mod query;
#[cfg(test)]
mod tests;

pub use self::header::Header as AxTreeHeader;
pub use self::offsetmap_or_default::*;

type TagIndex = cbor_tag_index::TagIndex<actyxos_sdk::Tag>;
