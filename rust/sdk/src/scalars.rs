/*
 * Copyright 2021 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use std::{
    convert::TryFrom,
    fmt::{self, Debug, Display},
    io::{Read, Seek, Write},
    str::FromStr,
};

use anyhow::{anyhow, bail, Context, Result};
use libipld::{
    cbor::DagCborCodec,
    codec::{Decode, Encode},
    DagCbor, Ipld,
};
use serde::{Deserialize, Serialize};

use crate::ParseError;

/// Macro for constructing an [`AppId`](struct.AppId.html) literal.
///
/// This is how it works:
/// ```no_run
/// use actyx_sdk::{app_id, AppId};
/// let app_id: AppId = app_id!("abc");
/// ```
/// This does not compile:
/// ```compile_fail
/// use actyx_sdk::{app_id, AppId};
/// let app_id: AppId = app_id!("");
/// ```
#[macro_export]
macro_rules! app_id {
    ($lit:tt) => {{
        #[allow(dead_code)]
        type X = $crate::assert_len!($lit, 1..);
        use ::std::convert::TryFrom;
        $crate::AppId::try_from($lit).unwrap()
    }};
}

mk_scalar!(
    /// The app ID denotes a specific app (sans versioning)
    ///
    /// This is used for marking the provenance of events as well as configuring access rights.
    struct AppId, ParseError, crate::scalar::validate_app_id, "crate::scalar::app_id_string"
);

#[cfg(any(test, feature = "arb"))]
impl quickcheck::Arbitrary for AppId {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        use once_cell::sync::OnceCell;
        static CHOICES: OnceCell<Vec<char>> = OnceCell::new();

        let choices = CHOICES.get_or_init(|| {
            ('a'..='z')
                .chain('0'..='9')
                .chain(std::iter::once('-'))
                .collect::<Vec<_>>()
        });
        let s = Vec::<bool>::arbitrary(g)
            .into_iter()
            .map(|_| *g.choose(&choices).unwrap())
            .collect::<String>();
        AppId::try_from(s.as_str()).unwrap_or_else(|_| app_id!("empty"))
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(self.0.shrink().filter(|x| crate::scalar::is_app_id(x)).map(Self))
    }
}

/// The Actyx node identifier
///
/// Each Actyx node has a private key that defines its identity. The corresponding public
/// key uniquely identifies the node but depends on the used crypto scheme. For now, we are
/// using ed25519.
///
/// So the node ID is just the 32 bytes of ed25519 public key. Nevertheless you should treat it
/// as an opaque value.
///
/// The bits of a node ID should not be assumed to be entirely evenly distributed, so if need
/// an even distribution for some reason, you would have to hash it.
///
/// Values of this type serialize as Base64url multibase strings by default.
/// Deserialization is supported from binary data or multibase format.
///
/// Each node may emit multiple sources, each identified by its own [`StreamId`](struct.StreamId.html).
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(into = "String", try_from = "String")]
pub struct NodeId(pub(crate) [u8; 32]);

impl NodeId {
    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Result<NodeId> {
        if bytes.len() == 32 {
            let mut bits: [u8; 32] = [0u8; 32];
            bits.copy_from_slice(&bytes[..32]);

            Ok(Self(bits))
        } else {
            Err(anyhow!("invalid NodeId length: {}", bytes.len()))
        }
    }

    /// Creates a [`StreamId`](struct.StreamId.html) belonging to this node ID with the given stream number
    pub fn stream(&self, stream_nr: StreamNr) -> StreamId {
        StreamId {
            node_id: *self,
            stream_nr,
        }
    }

    /// parse a `NodeId` using the crypt alphabet, which is order preserving
    fn parse(text: &str) -> Result<NodeId> {
        let config = base64::Config::new(base64::CharacterSet::Crypt, false);
        let bytes = base64::decode_config(text, config)?;
        Self::from_bytes(&bytes)
    }

    /// format a nodeid using the crypt alphabet, which is order preserving
    fn format(&self) -> String {
        let config = base64::Config::new(base64::CharacterSet::Crypt, false);
        base64::encode_config(self.0, config)
    }
}

impl AsRef<[u8]> for NodeId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.format())
    }
}

impl Debug for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // We could in theory also print node ID differently based on the upper 16 bytes
        // being zero, but that should ideally never be relevant.
        write!(f, "NodeId({})", self)
    }
}

impl From<NodeId> for String {
    fn from(node_id: NodeId) -> String {
        node_id.to_string()
    }
}

impl TryFrom<String> for NodeId {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl FromStr for NodeId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Encode<DagCborCodec> for NodeId {
    fn encode<W: Write>(&self, c: DagCborCodec, w: &mut W) -> anyhow::Result<()> {
        self.0.encode(c, w)
    }
}

impl Decode<DagCborCodec> for NodeId {
    fn decode<R: Read + Seek>(c: DagCborCodec, r: &mut R) -> libipld::Result<Self> {
        if let Ipld::Bytes(raw) = Ipld::decode(c, r)? {
            NodeId::from_bytes(&raw)
        } else {
            anyhow::bail!("unexpected cbor")
        }
    }
}

/// The unique identifier of a single event stream emitted by an Actyx node
///
/// The emitting node — identified by its [`NodeId`](struct.NodeId.html) — may emit multiple
/// streams with different IDs. The emitting node’s ID can be extracted from this stream ID
/// without further information.
///
/// The default serialization of this type is the string representation of the `NodeId`
/// followed by a dot and a base64url multibase-encoded multiformats-varint (see also
/// [`varint`](types/varint/index.html)).
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(into = "String", try_from = "String")]
pub struct StreamId {
    pub node_id: NodeId,
    pub stream_nr: StreamNr,
}

impl StreamId {
    pub fn min() -> Self {
        Self {
            node_id: NodeId([0; 32]),
            stream_nr: 0.into(),
        }
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn stream_nr(&self) -> StreamNr {
        self.stream_nr
    }

    fn parse_str(value: &str) -> Result<Self> {
        let mut split = value.split('-');
        let node_str = split
            .next()
            .ok_or_else(|| anyhow!("no NodeId in serialized StreamId"))?;
        let stream_str = split
            .next()
            .ok_or_else(|| anyhow!("no stream nr in serialized StreamId"))?;
        if split.next().is_some() {
            bail!("trailing garbage in StreamId");
        }
        let node_id: NodeId = NodeId::parse(node_str)?;
        let stream_nr = stream_str
            .parse::<u64>()
            .context("parsing StreamId stream number")?
            .into();
        Ok(Self { node_id, stream_nr })
    }
}

impl Display for StreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.node_id, self.stream_nr)
    }
}

impl Debug for StreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StreamId({})", self)
    }
}

impl Encode<DagCborCodec> for StreamId {
    fn encode<W: Write>(&self, c: DagCborCodec, w: &mut W) -> libipld::Result<()> {
        (self.node_id, self.stream_nr).encode(c, w)
    }
}

impl Decode<DagCborCodec> for StreamId {
    fn decode<R: Read + Seek>(c: DagCborCodec, r: &mut R) -> libipld::Result<Self> {
        let (node_id, stream_nr) = <(NodeId, StreamNr)>::decode(c, r)?;
        Ok(StreamId { node_id, stream_nr })
    }
}

impl From<StreamId> for String {
    fn from(sid: StreamId) -> String {
        sid.to_string()
    }
}

impl std::str::FromStr for StreamId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s).context("parsing StreamId")
    }
}

impl TryFrom<String> for StreamId {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse_str(&value).context("parsing StreamId")
    }
}
#[cfg(feature = "sqlite")]
mod sqlite {
    use super::*;
    use rusqlite::{
        types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef},
        ToSql,
    };

    impl FromSql for StreamId {
        fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
            value
                .as_str()
                .and_then(|s| s.parse::<StreamId>().map_err(|_| FromSqlError::InvalidType))
        }
    }

    impl ToSql for StreamId {
        fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
            Ok(ToSqlOutput::from(self.to_string()))
        }
    }
}

/// Stream number. Newtype alias for `u64`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, DagCbor, Default)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[ipld(repr = "value")]
pub struct StreamNr(u64);

impl From<u64> for StreamNr {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<StreamNr> for u64 {
    fn from(value: StreamNr) -> Self {
        value.0
    }
}

impl fmt::Display for StreamNr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use super::*;
    use libipld::{codec::assert_roundtrip, ipld};
    use serde_json::Value;

    const BYTES: [u8; 32] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
        31, 32,
    ];

    #[test]
    fn stream_nr_libipld() {
        assert_roundtrip(DagCborCodec, &StreamNr::from(0), &ipld!(0));
        assert_roundtrip(DagCborCodec, &StreamNr::from(1), &ipld!(1));
    }

    #[test]
    fn node_id_libipld() {
        let node_id = NodeId(BYTES);
        assert_roundtrip(DagCborCodec, &node_id, &ipld!(BYTES.as_ref()));
    }

    #[test]
    fn stream_id_libipld() {
        let stream_id = NodeId(BYTES).stream(12.try_into().unwrap());
        assert_roundtrip(
            DagCborCodec,
            &stream_id,
            &Ipld::List(vec![Ipld::Bytes(BYTES.to_vec()), Ipld::Integer(12)]),
        );
    }

    #[test]
    fn node_id_serialization() {
        let node_id = NodeId(BYTES);
        assert_eq!(node_id.to_string(), ".E61/.I4/kU70UgA1EsD2/2G2lEJ3VQM4FcP5/oS5m.");
    }

    #[test]
    fn stream_id_serialization() {
        let stream_id = NodeId(BYTES).stream(12.try_into().unwrap());
        assert_eq!(stream_id.to_string(), ".E61/.I4/kU70UgA1EsD2/2G2lEJ3VQM4FcP5/oS5m.-12");
    }

    #[test]
    fn quick1() {
        let sid = StreamId {
            node_id: NodeId([
                81, 66, 94, 87, 52, 39, 60, 110, 43, 93, 98, 94, 97, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0,
            ]),
            stream_nr: 0.into(),
        };
        assert_eq!(serde_json::to_value(&sid).unwrap(), Value::String(sid.to_string()));
    }

    quickcheck::quickcheck! {
        fn node_id_roundtrip(id: NodeId) -> bool {
            let s = id.to_string();
            s.parse().map_err(|_|"") == Ok(id)
        }

        fn stream_id_roundtrip(sid: StreamId) -> bool {
            let s = sid.to_string();
            StreamId::try_from(s).map_err(|_| "") == Ok(sid)
        }

        fn stream_id_to_string(sid: StreamId) -> bool {
            serde_json::to_value(&sid).unwrap() == Value::String(sid.to_string())
        }

        fn node_id_ord_vs_string_ord(a: NodeId, b: NodeId) -> bool {
            let a_to_b = a.cmp(&b);
            let as_to_bs = a.to_string().cmp(&b.to_string());
            a_to_b == as_to_bs
        }
    }
}
