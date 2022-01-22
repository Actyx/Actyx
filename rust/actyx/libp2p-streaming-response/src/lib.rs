use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

pub mod v1;
pub mod v2;

/// A [`Codec`] defines the request and response types for a [`StreamingResponse`]
/// protocol. Request and responses are encoded / decoded using `serde_cbor`, so
/// `Serialize` and `Deserialize` impls have to be provided. Implement this trait
/// to specialize the [`StreamingResponse`].
pub trait Codec {
    type Request: Send + Serialize + DeserializeOwned + std::fmt::Debug + 'static;
    type Response: Send + Serialize + DeserializeOwned + std::fmt::Debug;

    fn protocol_info() -> &'static [u8];
}

#[derive(Error, Debug)]
pub enum StreamingResponseError {
    #[error("Channel closed")]
    ChannelClosed,
}
pub(crate) type Result<T> = std::result::Result<T, StreamingResponseError>;

pub fn x() {
    println!();
}
