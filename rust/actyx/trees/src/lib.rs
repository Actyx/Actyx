#![deny(clippy::future_not_send)]
#[cfg(any(test, feature = "arb"))]
mod arb;
pub mod axtrees;
// Using some traits or similar the DNF module could be extracted into a separated crate
// This way I wouldn't need to make it pub here and could just be imported on demand
// But the overhead of maintaining it :/
pub mod dnf;
mod header;
pub mod query;
pub mod tags;
#[cfg(test)]
mod tests;

pub use self::header::Header as AxTreeHeader;

type TagIndex = cbor_tag_index::TagIndex<ScopedTag>;

#[derive(Debug, Clone)]
pub struct StoreParams;
impl libipld::store::StoreParams for StoreParams {
    type Hashes = libipld::multihash::Code;
    type Codecs = libipld::IpldCodec;
    const MAX_BLOCK_SIZE: usize = 2_000_000;
}

/// Type alias for the actyx flavour of banyan trees
pub type AxTree = banyan::Tree<axtrees::AxTrees, actyx_sdk::Payload>;
/// Type alias for builders
pub type AxStreamBuilder = banyan::StreamBuilder<axtrees::AxTrees, actyx_sdk::Payload>;
/// Type alias for links
pub type AxLink = axtrees::Sha256Digest;
/// Actyx event key
pub use axtrees::AxKey;
use tags::ScopedTag;
