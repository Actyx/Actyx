use crate::Codec;
use derive_more::{Display, Error, From};
use futures::{channel::mpsc, future::BoxFuture, AsyncReadExt, AsyncWriteExt, FutureExt};
use libp2p::{
    core::{upgrade::NegotiationError, UpgradeInfo},
    swarm::NegotiatedSubstream,
    InboundUpgrade, OutboundUpgrade,
};
use std::{
    iter::{once, Once},
    marker::PhantomData,
};

#[derive(Error, Display, Debug, From)]
pub enum ProtocolError {
    #[display(fmt = "timeout while waiting for request receive confirmation")]
    Timeout,
    #[display(fmt = "message too large received: {}", _0)]
    #[from(ignore)]
    MessageTooLargeRecv(#[error(ignore)] usize),
    #[display(fmt = "message too large sent: {}", _0)]
    #[from(ignore)]
    MessageTooLargeSent(#[error(ignore)] usize),
    #[display(fmt = "substream protocol negotiation error")]
    Negotiation(NegotiationError),
    #[display(fmt = "I/O error")]
    Io(std::io::Error),
    #[display(fmt = "(de)serialisation error")]
    Serde(serde_cbor::Error),
    #[display(fmt = "internal channel error:")]
    Channel(mpsc::SendError),
}

#[derive(Debug)]
pub struct Responder<T> {
    max_message_size: u32,
    _ph: PhantomData<T>,
}

impl<T> Responder<T> {
    pub fn new(max_message_size: u32) -> Self {
        Self {
            max_message_size,
            _ph: PhantomData,
        }
    }
}

impl<T: Codec> UpgradeInfo for Responder<T> {
    type Info = Vec<u8>;
    type InfoIter = Once<Vec<u8>>;

    fn protocol_info(&self) -> Self::InfoIter {
        let p = T::protocol_info();
        let mut v = Vec::with_capacity(p.len() + 4);
        v.extend(p);
        v.extend(b"/rs2");
        once(v)
    }
}

impl<T: Codec> InboundUpgrade<NegotiatedSubstream> for Responder<T> {
    type Output = (T::Request, NegotiatedSubstream);
    type Error = ProtocolError;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_inbound(self, mut socket: NegotiatedSubstream, _info: Self::Info) -> Self::Future {
        let max_message_size = self.max_message_size;
        async move {
            let mut size_bytes = [0u8; 4];
            socket.read_exact(&mut size_bytes).await?;
            let size = u32::from_be_bytes(size_bytes);

            if size > max_message_size {
                return Err(ProtocolError::MessageTooLargeRecv(size as usize));
            }

            let mut msg_bytes = vec![0u8; size as usize];
            socket.read_exact(msg_bytes.as_mut_slice()).await?;
            let msg = serde_cbor::from_slice(msg_bytes.as_slice())?;
            Ok((msg, socket))
        }
        .boxed()
    }
}

#[derive(Debug)]
pub struct Requester<T: Codec> {
    max_message_size: u32,
    request: T::Request,
}

impl<T: Codec> Requester<T> {
    pub fn new(max_message_size: u32, request: T::Request) -> Self {
        Self {
            max_message_size,
            request,
        }
    }
}

impl<T: Codec> UpgradeInfo for Requester<T> {
    type Info = Vec<u8>;
    type InfoIter = Once<Vec<u8>>;

    fn protocol_info(&self) -> Self::InfoIter {
        let p = T::protocol_info();
        let mut v = Vec::with_capacity(p.len() + 4);
        v.extend(p);
        v.extend(b"/rs2");
        once(v)
    }
}

impl<T: Codec> OutboundUpgrade<NegotiatedSubstream> for Requester<T> {
    type Output = NegotiatedSubstream;
    type Error = ProtocolError;
    type Future = BoxFuture<'static, Result<Self::Output, Self::Error>>;

    fn upgrade_outbound(self, mut socket: NegotiatedSubstream, _info: Self::Info) -> Self::Future {
        let Self {
            max_message_size,
            request,
        } = self;
        async move {
            let msg_bytes = serde_cbor::to_vec(&request)?;
            let size = msg_bytes.len();
            if size > (max_message_size as usize) {
                return Err(ProtocolError::MessageTooLargeSent(size));
            }
            let size_bytes = (size as u32).to_be_bytes();
            socket.write_all(&size_bytes).await?;
            socket.write_all(msg_bytes.as_slice()).await?;
            Ok(socket)
        }
        .boxed()
    }
}
