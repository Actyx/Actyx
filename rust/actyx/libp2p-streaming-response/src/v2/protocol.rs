use crate::Codec;
use derive_more::{Display, Error, From};
use futures::{channel::mpsc, future::BoxFuture, AsyncReadExt, AsyncWriteExt, FutureExt};
use libp2p::{core::UpgradeInfo, swarm::NegotiatedSubstream, InboundUpgrade, OutboundUpgrade};
use std::{
    iter::{once, Once},
    marker::PhantomData,
};

#[derive(Error, Display, Debug, From)]
pub enum ProtocolError {
    Io(std::io::Error),
    Serde(serde_cbor::Error),
    Channel(mpsc::SendError),
}

#[derive(Debug)]
pub struct Responder<T>(PhantomData<T>);

impl<T> Responder<T> {
    pub fn new() -> Self {
        Self(PhantomData)
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
        async move {
            let mut size_bytes = [0u8; 4];
            socket.read_exact(&mut size_bytes).await?;
            let size = u32::from_be_bytes(size_bytes) as usize;
            let mut msg_bytes = vec![0u8; size];
            socket.read_exact(msg_bytes.as_mut_slice()).await?;
            let msg = serde_cbor::from_slice(msg_bytes.as_slice())?;
            Ok((msg, socket))
        }
        .boxed()
    }
}

#[derive(Debug)]
pub struct Requester<T: Codec>(T::Request);

impl<T: Codec> Requester<T> {
    pub fn new(req: T::Request) -> Self {
        Self(req)
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
        let req = self.0;
        async move {
            let msg_bytes = serde_cbor::to_vec(&req)?;
            let size_bytes = (msg_bytes.len() as u32).to_be_bytes();
            socket.write_all(&size_bytes).await?;
            socket.write_all(msg_bytes.as_slice()).await?;
            Ok(socket)
        }
        .boxed()
    }
}
