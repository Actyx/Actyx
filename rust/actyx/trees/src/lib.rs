#![deny(clippy::future_not_send)]
#[cfg(any(test, feature = "arb"))]
mod arb;
pub mod axtrees;
mod dnf;
mod header;
pub mod query;
pub mod tags;
#[cfg(test)]
mod tests;

pub use self::header::Header as AxTreeHeader;

type TagIndex = cbor_tag_index::TagIndex<ScopedTag>;

/// Type alias for the actyx flavour of banyan trees
pub type AxTree = banyan::Tree<axtrees::AxTrees, actyx_sdk::Payload>;
/// Type alias for builders
pub type AxStreamBuilder = banyan::StreamBuilder<axtrees::AxTrees, actyx_sdk::Payload>;
/// Type alias for links
pub type AxLink = axtrees::Sha256Digest;
/// Actyx event key
pub use axtrees::AxKey;
use tags::ScopedTag;
