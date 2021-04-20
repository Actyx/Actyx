//! Collection of cryptographic functions under a unified API
//!
//! Currently only Ed25519 elliptic curve cryptography is supported, but the design in API
//! and persistent storage is such that it can be extended later.
//!
//! # Assumptions
//!
//! ## Identification by SHA256
//!
//! The most important assumption is that keys are uniquely identified by a SHA256 hash.
//! Hash collisions would break the code, e.g. by overwriting entries in maps.
//!
//! This assumption is a reasonable one to make because SHA256 has the same width as the
//! underlying material that is hashed, therefore it should be impossible to manufacture
//! collisions even if SHA256 is broken in the future (in particular since attacks normally
//! involve adding more bits, which is impossible in this setting).
//!
//! ## Handling of key material
//!
//! `ed25519_dalek` takes care to zero private keys when their memory is released, hence we
//! use their types to reuse this behavior.
//!
//! We assume that the `.as_bytes()` representation is and remains compatible between
//! `ed25519_dalek` and `x25519_dalek` so that we can use the same key pair for signing
//! and encryption.
//!
//! # Versioning
//!
//! Creating more capable versions of the KeyStore can be serialized in backwards-compatible
//! fashion by adding a version field.
//!
//! Signed messages support up to u32::MAX bytes for the message, prefixed with the length,
//! and followed by a concatenation of signatures. Each signature is classified by the first
//! byte such that its length can be deduced by consumers.

mod dh;
mod keystore;
mod pair;
mod private;
mod public;
mod signature;

pub use keystore::{KeyStore, KeyStoreRef};
pub use pair::KeyPair;
pub use private::PrivateKey;
pub use public::PublicKey;
pub use signature::SignedMessage;
