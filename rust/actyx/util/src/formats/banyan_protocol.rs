use actyx_sdk::{AppId, NodeId, Payload, Tag, TagSet, Timestamp};
use cbor_data::{index_str, value::Number, Cbor, CborBuilder, Encoder};
use chrono::{DateTime, FixedOffset};
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use libp2p::{core::ProtocolName, request_response::RequestResponseCodec};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    convert::TryFrom,
    io::{Error, ErrorKind, Result},
};

#[derive(Clone, PartialEq)]
pub enum BanyanRequest {
    MakeFreshTopic(String),
    AppendEvents(String, Vec<u8>),
    Finalise(String),
    Future,
}

impl std::fmt::Debug for BanyanRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MakeFreshTopic(arg0) => f.debug_tuple("MakeFreshTopic").field(arg0).finish(),
            Self::AppendEvents(arg0, arg1) => f.debug_tuple("AppendEvents").field(arg0).field(&arg1.len()).finish(),
            Self::Finalise(arg0) => f.debug_tuple("Finalise").field(arg0).finish(),
            Self::Future => write!(f, "Future"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BanyanResponse {
    Ok,
    Error(String),
    Future,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct BanyanProtocolName;
impl ProtocolName for BanyanProtocolName {
    fn protocol_name(&self) -> &[u8] {
        b"/actyx/banyan/create"
    }
}

#[derive(Default, Clone, Debug)]
pub struct BanyanProtocol {
    buf: Vec<u8>,
}

#[async_trait::async_trait]
impl RequestResponseCodec for BanyanProtocol {
    type Protocol = BanyanProtocolName;
    type Request = BanyanRequest;
    type Response = BanyanResponse;

    async fn read_request<T: AsyncRead + Send + Unpin>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> Result<Self::Request> {
        let mut len_bytes = [0u8; 4];
        io.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes);

        self.buf.resize(len as usize, 0);
        io.read_exact(self.buf.as_mut()).await?;

        let cbor = Cbor::checked(self.buf.as_ref()).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
        (|| {
            let arr = cbor.decode().to_array()?;
            match arr.get(0)?.decode().to_number()? {
                Number::Int(1) => Some(BanyanRequest::MakeFreshTopic(
                    arr.get(1)?.decode().to_str()?.into_owned(),
                )),
                Number::Int(2) => Some(BanyanRequest::AppendEvents(
                    arr.get(1)?.decode().to_str()?.into_owned(),
                    arr.get(2)?.decode().to_bytes()?.into_owned(),
                )),
                Number::Int(3) => Some(BanyanRequest::Finalise(arr.get(1)?.decode().to_str()?.into_owned())),
                _ => Some(BanyanRequest::Future),
            }
        })()
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "expected array"))
    }

    async fn read_response<T: AsyncRead + Send + Unpin>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> Result<Self::Response> {
        let mut len_bytes = [0u8; 4];
        io.read_exact(&mut len_bytes).await?;
        let len = u32::from_be_bytes(len_bytes);

        self.buf.resize(len as usize, 0);
        io.read_exact(self.buf.as_mut()).await?;

        let cbor = Cbor::checked(self.buf.as_ref()).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
        (|| {
            let arr = cbor.decode().to_array()?;
            match arr.get(0)?.decode().to_number()? {
                Number::Int(1) => Some(BanyanResponse::Ok),
                Number::Int(2) => Some(BanyanResponse::Error(arr.get(1)?.decode().to_str()?.into_owned())),
                _ => Some(BanyanResponse::Future),
            }
        })()
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "expected array"))
    }

    async fn write_request<T: AsyncWrite + Send + Unpin>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> Result<()> {
        let cbor = CborBuilder::with_scratch_space(&mut self.buf).encode_array(move |b| match req {
            BanyanRequest::MakeFreshTopic(topic) => {
                b.encode_u64(1);
                b.encode_str(topic);
            }
            BanyanRequest::AppendEvents(topic, data) => {
                b.encode_u64(2);
                b.encode_str(topic);
                b.encode_bytes(data);
            }
            BanyanRequest::Finalise(topic) => {
                b.encode_u64(3);
                b.encode_str(topic);
            }
            BanyanRequest::Future => unreachable!(),
        });
        let len_bytes = u32::try_from(cbor.as_slice().len())
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "cannot enocde more than 4GiB"))?
            .to_be_bytes();
        io.write_all(&len_bytes).await?;
        io.write_all(cbor.as_slice()).await?;
        Ok(())
    }

    async fn write_response<T: AsyncWrite + Send + Unpin>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> Result<()> {
        let cbor = CborBuilder::with_scratch_space(&mut self.buf).encode_array(move |b| match res {
            BanyanResponse::Ok => {
                b.encode_u64(1);
            }
            BanyanResponse::Error(error) => {
                b.encode_u64(2);
                b.encode_str(error);
            }
            BanyanResponse::Future => unreachable!(),
        });
        let len_bytes = u32::try_from(cbor.as_slice().len())
            .map_err(|_| Error::new(ErrorKind::InvalidInput, "cannot enocde more than 4GiB"))?
            .to_be_bytes();
        io.write_all(&len_bytes).await?;
        io.write_all(cbor.as_slice()).await?;
        Ok(())
    }
}

pub fn decode_dump_frame(cbor: &Cbor) -> Option<(NodeId, AppId, Timestamp, TagSet, Payload)> {
    let orig_node = NodeId::from_bytes(cbor.index(index_str("stream[0]"))?.decode().to_bytes()?.as_ref()).ok()?;
    let app_id = AppId::try_from(cbor.index(index_str("appId"))?.decode().to_str()?.as_ref()).ok()?;
    let timestamp = cbor.index(index_str("timestamp"))?;
    let timestamp = match timestamp.decode().to_number()? {
        Number::Int(t) => Timestamp::from(u64::try_from(t).ok()?),
        _ => return None,
    };
    let tags = cbor.index(index_str("tags"))?.decode().to_array().map(|tags| {
        tags.into_iter()
            .filter_map(|cbor| cbor.decode().to_str().and_then(|s| Tag::try_from(s.as_ref()).ok()))
            .collect()
    })?;
    let payload = Payload::from_bytes(cbor.index(index_str("payload"))?.as_slice());
    Some((orig_node, app_id, timestamp, tags, payload))
}

pub fn decode_dump_header(cbor: &Cbor) -> Option<(NodeId, String, DateTime<FixedOffset>)> {
    let dict = cbor.decode().to_dict()?;
    let dict = dict
        .iter()
        .filter_map(|(k, v)| k.decode().to_str().map(|k| (k, v.decode())))
        .collect::<BTreeMap<_, _>>();
    let node_id = NodeId::from_bytes(dict.get("nodeId")?.as_bytes()?.as_ref()).ok()?;
    let display_name = dict.get("displayName")?.as_str()?;
    let timestamp = DateTime::<FixedOffset>::try_from(dict.get("timestamp")?.as_timestamp()?).ok()?;
    let settings = dict.get("settings")?.as_str()?;
    let topic = serde_json::from_str::<Value>(settings.as_ref())
        .ok()?
        .pointer("/swarm/topic")?
        .as_str()?
        .to_owned();
    tracing::info!(
        "reading dump from `{}` (topic {}) taken at {}",
        display_name,
        topic,
        timestamp
    );
    Some((node_id, topic, timestamp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use BanyanRequest::*;
    use BanyanResponse::*;

    #[tokio::test]
    async fn roundtrip() {
        let mut p = BanyanProtocol::default();
        let c = BanyanProtocolName;
        let mut v = Vec::new();

        p.write_request(&c, &mut v, MakeFreshTopic("hello".into()))
            .await
            .unwrap();
        let req = p.read_request(&c, &mut v.as_slice()).await.unwrap();
        assert_eq!(req, MakeFreshTopic("hello".into()));

        v.clear();
        p.write_request(&c, &mut v, AppendEvents("hello".into(), vec![1, 2, 3, 4, 5]))
            .await
            .unwrap();
        let req = p.read_request(&c, &mut v.as_slice()).await.unwrap();
        assert_eq!(req, AppendEvents("hello".into(), vec![1, 2, 3, 4, 5]));

        v.clear();
        p.write_request(&c, &mut v, Finalise("hello".into())).await.unwrap();
        let req = p.read_request(&c, &mut v.as_slice()).await.unwrap();
        assert_eq!(req, Finalise("hello".into()));

        let cbor = CborBuilder::new().encode_array(|b| {
            b.encode_u64(42);
            b.encode_null();
        });
        v.clear();
        v.extend_from_slice(&(cbor.as_slice().len() as u32).to_be_bytes());
        v.extend_from_slice(cbor.as_slice());
        let req = p.read_request(&c, &mut v.as_slice()).await.unwrap();
        assert_eq!(req, BanyanRequest::Future);

        v.clear();
        p.write_response(&c, &mut v, Ok).await.unwrap();
        let res = p.read_response(&c, &mut v.as_slice()).await.unwrap();
        assert_eq!(res, Ok);

        v.clear();
        p.write_response(&c, &mut v, Error("soso".into())).await.unwrap();
        let res = p.read_response(&c, &mut v.as_slice()).await.unwrap();
        assert_eq!(res, Error("soso".into()));

        v.clear();
        v.extend_from_slice(&(cbor.as_slice().len() as u32).to_be_bytes());
        v.extend_from_slice(cbor.as_slice());
        let req = p.read_request(&c, &mut v.as_slice()).await.unwrap();
        assert_eq!(req, BanyanRequest::Future);
    }
}
