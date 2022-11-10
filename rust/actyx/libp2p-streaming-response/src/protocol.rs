use derive_more::{Add, Deref, Display, Sub};
use futures::{
    future::BoxFuture,
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
};
use libp2p::core::{upgrade, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    io::{self, Error, ErrorKind, Result},
    marker::PhantomData,
};

/// A [`Codec`] defines the request and response types for a [`StreamingResponse`]
/// protocol. Request and responses are encoded / decoded using `serde_cbor`, so
/// `Serialize` and `Deserialize` impls have to be provided. Implement this trait
/// to specialize the [`StreamingResponse`].
pub trait Codec {
    type Request: Send + Serialize + DeserializeOwned + std::fmt::Debug;
    type Response: Send + Serialize + DeserializeOwned + std::fmt::Debug;

    fn protocol_info() -> &'static [u8];
}

/// Local requestId
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct RequestId(pub(crate) u64);

#[derive(
    Debug, Serialize, Deserialize, Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Display, Add, Sub, Deref,
)]
// SequenceNo for responses
pub struct SequenceNo(pub(crate) u64);
impl SequenceNo {
    pub fn increment(&mut self) {
        self.0 += 1
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StreamingResponseMessage<TCodec: Codec> {
    /// Initiate a request
    Request { id: RequestId, payload: TCodec::Request },
    /// Cancel an ongoing request
    CancelRequest { id: RequestId },
    /// Single response frame
    Response {
        id: RequestId,
        seq_no: SequenceNo,
        payload: TCodec::Response,
    },
    /// Response ended
    ResponseEnd { id: RequestId, seq_no: SequenceNo },
}

#[derive(Clone, Debug)]
pub struct StreamingResponseConfig<TCodec: Codec> {
    /// Maximum size in bytes accepted for incoming requests
    max_buf_size: usize,
    _c: PhantomData<TCodec>,
}

impl<TCodec> Default for StreamingResponseConfig<TCodec>
where
    TCodec: Codec,
{
    fn default() -> Self {
        Self {
            max_buf_size: 1024 * 1024 * 4,
            _c: PhantomData,
        }
    }
}

impl<TCodec> UpgradeInfo for StreamingResponseConfig<TCodec>
where
    TCodec: Codec,
{
    type Info = &'static [u8];
    type InfoIter = std::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        std::iter::once(TCodec::protocol_info())
    }
}

impl<TSocket, TCodec> InboundUpgrade<TSocket> for StreamingResponseConfig<TCodec>
where
    TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    TCodec: Codec + Send + 'static,
{
    type Output = StreamingResponseMessage<TCodec>;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output>>;

    fn upgrade_inbound(self, mut socket: TSocket, _info: Self::Info) -> Self::Future {
        Box::pin(async move {
            let packet = upgrade::read_length_prefixed(&mut socket, self.max_buf_size).await?;
            let request = serde_cbor::from_slice(&packet).map_err(|e| io::Error::new(ErrorKind::InvalidInput, e))?;
            socket.close().await?;
            Ok(request)
        })
    }
}

impl<TCodec> UpgradeInfo for StreamingResponseMessage<TCodec>
where
    TCodec: Codec,
{
    type Info = &'static [u8];
    type InfoIter = std::iter::Once<Self::Info>;

    fn protocol_info(&self) -> Self::InfoIter {
        std::iter::once(TCodec::protocol_info())
    }
}

impl<TSocket, TCodec> OutboundUpgrade<TSocket> for StreamingResponseMessage<TCodec>
where
    TCodec: Codec + 'static,
    TSocket: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = ();
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Output>>;

    fn upgrade_outbound(self, mut socket: TSocket, _info: Self::Info) -> Self::Future {
        Box::pin(async move {
            let bytes = serde_cbor::to_vec(&self).map_err(|e| io::Error::new(ErrorKind::InvalidInput, e))?;
            upgrade::write_length_prefixed(&mut socket, bytes).await?;
            socket.close().await?;
            Ok(())
        })
    }
}
