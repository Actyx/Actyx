use crate::{Codec, SequenceNo};
use futures::{
    future::BoxFuture,
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
};
use libp2p::core::{upgrade, InboundUpgrade, OutboundUpgrade, UpgradeInfo};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Error, ErrorKind, Result},
    marker::PhantomData,
};

#[derive(Clone, Debug)]
pub struct StreamingResponseConfig<TCodec: Codec> {
    /// Maximum size in bytes accepted for incoming requests
    max_buf_size: usize,
    _c: PhantomData<TCodec>,
}

impl<TCodec: Codec> StreamingResponseConfig<TCodec> {
    pub fn new(max_buf_size: usize) -> Self {
        Self {
            max_buf_size,
            _c: PhantomData,
        }
    }
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

/// Local requestId
#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RequestId(u64);

impl RequestId {
    pub fn increment(&mut self) {
        self.0 += 1;
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

impl<T: Codec> StreamingResponseMessage<T> {
    pub fn id(&self) -> RequestId {
        match self {
            StreamingResponseMessage::Request { id, .. } => *id,
            StreamingResponseMessage::CancelRequest { id } => *id,
            StreamingResponseMessage::Response { id, .. } => *id,
            StreamingResponseMessage::ResponseEnd { id, .. } => *id,
        }
    }
}

impl<TCodec> UpgradeInfo for StreamingResponseConfig<TCodec>
where
    TCodec: Codec,
{
    type Info = &'static str;
    type InfoIter = core::array::IntoIter<Self::Info, 2>;

    fn protocol_info(&self) -> Self::InfoIter {
        TCodec::protocol_info().into_iter()
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
    type Info = &'static str;
    type InfoIter = core::array::IntoIter<Self::Info, 2>;

    fn protocol_info(&self) -> Self::InfoIter {
        TCodec::protocol_info().into_iter()
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
