use super::handler::Response;
use crate::libp2p_streaming_response::Codec;
use derive_more::{Display, Error, From};
use futures::{channel::mpsc, AsyncReadExt, AsyncWriteExt, Future};
use libp2p::{core::upgrade::NegotiationError, swarm::NegotiatedSubstream};
use serde::de::DeserializeOwned;
use std::io::ErrorKind;

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
    #[display(fmt = "substream protocol negotiation error: {}", _0)]
    Negotiation(NegotiationError),
    #[display(fmt = "I/O error: {}", _0)]
    Io(std::io::Error),
    #[display(fmt = "(de)serialisation error: {}", _0)]
    Serde(serde_cbor::Error),
    #[display(fmt = "internal channel error")]
    Channel(mpsc::SendError),
    /// This variant is useful for implementing the function to pass to
    /// [`with_spawner`](crate::libp2p_streaming_response::StreamingResponseConfig)
    #[display(fmt = "spawned task failed (cancelled={})", _0)]
    JoinError(#[error(ignore)] bool),
}

impl PartialEq for ProtocolError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::MessageTooLargeRecv(l0), Self::MessageTooLargeRecv(r0)) => l0 == r0,
            (Self::MessageTooLargeSent(l0), Self::MessageTooLargeSent(r0)) => l0 == r0,
            (Self::Negotiation(l0), Self::Negotiation(r0)) => l0.to_string() == r0.to_string(),
            (Self::Io(l0), Self::Io(r0)) => l0.to_string() == r0.to_string(),
            (Self::Serde(l0), Self::Serde(r0)) => l0.to_string() == r0.to_string(),
            (Self::Channel(l0), Self::Channel(r0)) => l0 == r0,
            (Self::JoinError(l0), Self::JoinError(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl ProtocolError {
    pub fn as_code(&self) -> u8 {
        match self {
            ProtocolError::Timeout => 1,
            ProtocolError::MessageTooLargeRecv(_) => 2,
            ProtocolError::MessageTooLargeSent(_) => 3,
            ProtocolError::Negotiation(_) => 4,
            ProtocolError::Io(_) => 5,
            ProtocolError::Serde(_) => 6,
            ProtocolError::Channel(_) => 7,
            ProtocolError::JoinError(_) => 8,
        }
    }
    pub fn from_code(code: u8) -> Self {
        match code {
            1 => ProtocolError::Timeout,
            2 => ProtocolError::MessageTooLargeRecv(0),
            3 => ProtocolError::MessageTooLargeSent(0),
            4 => ProtocolError::Negotiation(NegotiationError::Failed),
            5 => ProtocolError::Io(std::io::Error::new(ErrorKind::Other, "some error on peer")),
            6 => ProtocolError::Serde(std::io::Error::new(ErrorKind::Other, "serde error on peer").into()),
            7 => {
                let (mut tx, _) = mpsc::channel(1);
                let err = tx.try_send(0).unwrap_err().into_send_error();
                ProtocolError::Channel(err)
            }
            8 => ProtocolError::JoinError(false),
            n => ProtocolError::Io(std::io::Error::new(
                ErrorKind::Other,
                format!("unknown error code {}", n),
            )),
        }
    }
}

pub fn write_msg<'a>(
    io: &'a mut NegotiatedSubstream,
    msg: impl serde::Serialize,
    max_size: u32,
    buffer: &'a mut Vec<u8>,
) -> impl Future<Output = Result<(), ProtocolError>> + 'a {
    buffer.resize(4, 0);
    let res = serde_cbor::to_writer(&mut *buffer, &msg);
    async move {
        if let Err(e) = res {
            let err = ProtocolError::Serde(e);
            write_err(io, &err).await?;
            return Err(err);
        }
        let size = buffer.len() - 4;
        if size > (max_size as usize) {
            tracing::debug!("message size {} too large (max = {})", size, max_size);
            let err = ProtocolError::MessageTooLargeSent(size);
            write_err(io, &err).await?;
            return Err(err);
        }
        tracing::trace!("sending message of size {}", size);
        buffer.as_mut_slice()[..4].copy_from_slice(&(size as u32).to_be_bytes());
        io.write_all(buffer.as_slice()).await?;
        Ok(())
    }
}

pub async fn write_err(io: &mut NegotiatedSubstream, err: &ProtocolError) -> Result<(), std::io::Error> {
    let buf = [255, err.as_code()];
    io.write_all(&buf).await?;
    io.flush().await?;
    io.close().await?;
    Ok(())
}

pub async fn write_finish(io: &mut NegotiatedSubstream) -> Result<(), std::io::Error> {
    let buf = [255, 0];
    io.write_all(&buf).await?;
    io.flush().await?;
    io.close().await?;
    Ok(())
}

pub async fn read_msg<T: DeserializeOwned>(
    io: &mut NegotiatedSubstream,
    max_size: u32,
    buffer: &mut Vec<u8>,
) -> Result<Response<T>, ProtocolError> {
    let mut size_bytes = [0u8; 4];
    let mut to_read = &mut size_bytes[..];
    while !to_read.is_empty() {
        let read = io.read(to_read).await?;
        tracing::trace!("read {} header bytes", read);
        if read == 0 {
            let len = to_read.len();
            let read = &size_bytes[..4 - len];
            if read.len() != 2 || read[0] != 255 {
                return Err(ProtocolError::Io(ErrorKind::UnexpectedEof.into()));
            } else {
                return match read[1] {
                    0 => Ok(Response::Finished),
                    n => Err(ProtocolError::from_code(n)),
                };
            }
        }
        to_read = to_read.split_at_mut(read).1;
    }
    let size = u32::from_be_bytes(size_bytes);

    if size > max_size {
        tracing::debug!("message size {} too large (max = {})", size, max_size);
        let mut bytes = [0u8; 4096];
        bytes[..4].copy_from_slice(&size_bytes);
        let n = io.read(&mut bytes[4..]).await?;
        tracing::debug!("{:?}", &bytes[..n + 4]);
        return Err(ProtocolError::MessageTooLargeRecv(size as usize));
    }
    tracing::trace!("received header: msg is {} bytes", size);

    buffer.resize(size as usize, 0);
    io.read_exact(buffer.as_mut_slice()).await?;
    tracing::trace!("all bytes read");
    Ok(Response::Msg(serde_cbor::from_slice(buffer.as_slice())?))
}

pub async fn upgrade_inbound<T: Codec>(
    max_message_size: u32,
    mut socket: NegotiatedSubstream,
    proto: &'static str,
) -> Result<(T::Request, NegotiatedSubstream), ProtocolError> {
    tracing::trace!("starting inbound upgrade `{}`", proto);
    let msg = read_msg(&mut socket, max_message_size, &mut Vec::new())
        .await?
        .into_msg()?;
    tracing::trace!("request received: {:?}", msg);
    Ok((msg, socket))
}

pub async fn upgrade_outbound<T: Codec>(
    max_message_size: u32,
    request: T::Request,
    mut socket: NegotiatedSubstream,
    info: &'static str,
) -> Result<NegotiatedSubstream, ProtocolError> {
    tracing::trace!("starting output upgrade `{}`", info);
    write_msg(&mut socket, request, max_message_size, &mut Vec::new()).await?;
    socket.flush().await?;
    tracing::trace!("all bytes sent");
    Ok(socket)
}
