//! Bitswap protocol
//!
//! The bitswap protocol is pretty simple. There are only two kinds of queries with corresponding responses, and a cancel command
//! to stop processing a query.
//!
//! have queries are a question to the remote side if they have a block. The answer is just a boolean, so this is a relatively
//! cheap query in terms of bandwidth usage.
//!
//! want queries are queries for actual blocks, so depending on the block size these can be more expensive.
//!
//! cancel commands can be used to inform the remote side that information about a cid (have/presence or want/block) is no longer needed.
//!
//! It is possible to combine all queries and responses in a single protobuf message. In general, you can never rely on getting an answer
//! for a request, and you must also expect messages to come at any time, even when not requested.
use super::bitswap_pb;
use super::prefix::Prefix;
use crate::block::Block;
use cid::Cid;
use derive_more::{Display, Error, From};
use itertools::Itertools;
use prost::Message as ProstMessage;
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

/// number of cids in messages that contain cids, like have
const MAX_CIDS: usize = 100;
/// number of cids in want messages. You might want to make this smaller than MAX_CIDS
/// in order to not be overwhelmed with the response, and to avoid a single peer doing
/// all the work.
const MAX_WANT_CIDS: usize = 100;
/// the number of bytes above which we close a message full of data blocks
/// this is just a value that makes the tree sync test pass. We would have to research
/// the best value for this in depth to optimize things.
const MAX_BYTES: usize = 1000000;

/// marker type for incoming messages
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct I;
/// marker type for outgoing messages
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct O;

/// A bitswap message.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Message<T> {
    // requests
    /// List of blocks where we want the other party to immediately send the block.
    pub want: BTreeSet<Cid>,
    /// List of blocks where we want to know if the other party has it or not.
    pub have: BTreeSet<Cid>,
    /// List of blocks to cancel.
    pub cancel: BTreeSet<Cid>,
    /// Whether it is the full list of wanted blocks.
    ///
    /// I guess if this is true, the other side can completely replace its own list with it.
    pub full: bool,

    // responses
    /// List of blocks to send.
    pub blocks: BTreeSet<Block>,
    /// List of block presences to send.
    pub block_presences: BTreeMap<Cid, bool>,

    /// Message tag
    _phantom_data: PhantomData<T>,
}

impl<T> Default for Message<T> {
    fn default() -> Self {
        Message {
            want: Default::default(),
            have: Default::default(),
            block_presences: Default::default(),
            cancel: Default::default(),
            blocks: Default::default(),
            full: false,
            _phantom_data: PhantomData,
        }
    }
}

impl Message<O> {
    /// Create a message that is a have query for a set of cids
    ///
    /// We want the receiver to just tell us if it has the blocks, not to actually send them.
    /// We expect block presence as an answer, hence query.
    pub fn have_query(cids: impl Iterator<Item = Cid>) -> Vec<Self> {
        cids.chunks(MAX_CIDS)
            .into_iter()
            .map(|chunk| {
                let mut res = Message::new();
                res.have = chunk.collect();
                res
            })
            .collect()
    }

    /// create a message that is a want query for a set of cids
    ///
    /// we want the receiver to immediately send us the blocks if it has them.
    /// We expect blocks as an answer, hence query.
    pub fn want_query(cids: impl Iterator<Item = Cid>) -> Vec<Self> {
        cids.chunks(MAX_WANT_CIDS)
            .into_iter()
            .map(|chunk| {
                let mut res = Message::new();
                res.want = chunk.collect();
                res
            })
            .collect()
    }

    /// Create a message that is a cancel command for a set of cids
    ///
    /// We want the receiver to immediately stop processing any want or have requests for all the given cids.
    /// We don't expect an answer for this, hence command.
    pub fn cancel_command(cids: impl Iterator<Item = Cid>) -> Vec<Self> {
        cids.chunks(MAX_CIDS)
            .into_iter()
            .map(|chunk| {
                let mut res = Message::new();
                res.cancel = chunk.collect();
                res
            })
            .collect()
    }

    /// Send a number of blocks.
    ///
    /// This is a response to a want query
    pub fn want_response(blocks: impl Iterator<Item = Block>) -> Vec<Self> {
        let mut result = Vec::new();
        let mut size = 0;
        let mut current = None;
        for block in blocks {
            size += block.data().len();
            current.get_or_insert_with(Message::new).blocks.insert(block);
            if size > MAX_BYTES {
                result.extend(current.take());
                size = 0;
            }
        }
        result.extend(current.take());
        result
    }

    /// Send a number of block presences.
    ///
    /// This is a response to a have query
    pub fn have_response(block_presences: impl Iterator<Item = (Cid, bool)>) -> Vec<Self> {
        block_presences
            .chunks(MAX_CIDS)
            .into_iter()
            .map(|chunk| {
                let mut res = Self::new();
                res.block_presences = chunk.collect();
                res
            })
            .collect()
    }

    /// Flip from output to input message, just for testing
    #[cfg(test)]
    pub fn into_input(self) -> Message<I> {
        Message {
            _phantom_data: PhantomData,
            want: self.want,
            have: self.have,
            cancel: self.cancel,
            full: self.full,
            blocks: self.blocks,
            block_presences: self.block_presences,
        }
    }
}

impl<T> Message<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the list of blocks.
    pub fn blocks(&self) -> &BTreeSet<Block> {
        &self.blocks
    }

    /// Returns the list of blocks.
    pub fn block_presences(&self) -> &BTreeMap<Cid, bool> {
        &self.block_presences
    }

    /// Returns the list of wanted blocks.
    pub fn want(&self) -> &BTreeSet<Cid> {
        &self.want
    }

    /// Returns the list of blocks for which we want to have info.
    pub fn have(&self) -> &BTreeSet<Cid> {
        &self.have
    }

    /// Returns the list of cancelled blocks.
    pub fn cancel(&self) -> &BTreeSet<Cid> {
        &self.cancel
    }

    /// Adds blocks to this message
    fn add_blocks(&mut self, blocks: impl IntoIterator<Item = Block>) {
        self.blocks.extend(blocks);
    }

    /// Adds a `BlockPresence` to the message.
    fn add_block_presences(&mut self, info: impl IntoIterator<Item = (Cid, bool)>) {
        self.block_presences.extend(info);
    }

    /// Adds a block to the want list.
    fn add_want_blocks(&mut self, cids: impl IntoIterator<Item = Cid>) {
        self.want.extend(cids);
    }

    /// Adds a block to the want list.
    fn add_have_blocks(&mut self, cids: impl IntoIterator<Item = Cid>) {
        self.have.extend(cids);
    }

    /// Adds a block to the cancel list.
    pub fn add_cancel_block(&mut self, cid: &Cid) {
        self.cancel.insert(cid.to_owned());
    }

    /// Flip from input to output message, just for testing
    #[cfg(test)]
    pub fn into_output(self) -> Message<O> {
        Message {
            _phantom_data: PhantomData,
            want: self.want,
            have: self.have,
            cancel: self.cancel,
            full: self.full,
            blocks: self.blocks,
            block_presences: self.block_presences,
        }
    }
}

impl Into<Vec<u8>> for &Message<O> {
    fn into(self) -> Vec<u8> {
        use bitswap_pb::message::{self, wantlist::Entry, BlockPresence, Wantlist};
        let mut proto = bitswap_pb::Message::default();
        let mut wantlist = Wantlist::default();
        for cid in self.want() {
            let entry = Entry {
                block: cid.to_bytes(),
                priority: 0,
                want_type: WantType::Block.into(),
                ..Default::default()
            };
            wantlist.entries.push(entry);
        }
        for cid in self.have() {
            let entry = Entry {
                block: cid.to_bytes(),
                priority: 0,
                want_type: WantType::Have.into(),
                ..Default::default()
            };
            wantlist.entries.push(entry);
        }
        for cid in self.cancel() {
            let entry = Entry {
                block: cid.to_bytes(),
                cancel: true,
                ..Default::default()
            };
            wantlist.entries.push(entry);
        }
        for block in self.blocks() {
            let payload = message::Block {
                prefix: Prefix::from(block.cid()).to_bytes(),
                data: block.data().to_vec(),
            };
            proto.payload.push(payload);
        }
        for (cid, have) in self.block_presences() {
            let block_presence = BlockPresence {
                cid: cid.to_bytes(),
                r#type: if *have { 0 } else { 1 },
            };
            proto.block_presences.push(block_presence);
        }
        wantlist.full = self.full;
        proto.wantlist = Some(wantlist);
        let mut res = Vec::with_capacity(proto.encoded_len());
        proto
            .encode(&mut res)
            .expect("there is no situation in which the protobuf message can be invalid");
        res
    }
}

impl Message<O> {
    /// Turns this `Message` into a message that can be sent to a substream.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.into()
    }
}
#[derive(Debug, From, Display, Error)]
pub enum DecodeError {
    #[display(fmt = "Error while reading from socket: {}", _0)]
    ProtoError(prost::DecodeError),
    #[display(fmt = "Error while decoding message: {}", _0)]
    CidError(cid::Error),
    #[display(fmt = "unknown block_presence type")]
    UnknownBlockPresence,
    #[display(fmt = "unknown want type")]
    UnknownWantType,
}

impl TryFrom<&[u8]> for Message<I> {
    type Error = DecodeError;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        let proto: bitswap_pb::Message = bitswap_pb::Message::decode(bytes)?;
        let mut message = Message::new();
        let wantlist = proto.wantlist.unwrap_or_default();
        for entry in wantlist.entries {
            let cid = Cid::try_from(entry.block)?;
            let want_type = entry.want_type.try_into().map_err(|_| DecodeError::UnknownWantType)?;
            if entry.cancel {
                message.add_cancel_block(&cid);
            } else {
                match want_type {
                    WantType::Have => message.add_have_blocks(vec![cid]),
                    WantType::Block => message.add_want_blocks(vec![cid]),
                }
            }
        }
        let blocks: Result<Vec<Block>, DecodeError> = proto
            .payload
            .into_iter()
            .map(|payload| {
                let prefix = Prefix::new(&payload.prefix)?;
                let cid = prefix.to_cid(&payload.data)?;
                let block = Block::new(payload.data.to_vec(), cid);
                Ok(block)
            })
            .collect();
        let block_presences: Result<Vec<(Cid, bool)>, DecodeError> = proto
            .block_presences
            .into_iter()
            .map(|block_presence| {
                let cid = Cid::try_from(block_presence.cid)?;
                let have = match block_presence.r#type {
                    0 => true,
                    1 => false,
                    _ => return Err(DecodeError::UnknownBlockPresence),
                };
                Ok((cid, have))
            })
            .collect();
        message.add_blocks(blocks?);
        message.add_block_presences(block_presences?);
        message.full = wantlist.full;
        Ok(message)
    }
}

impl Message<I> {
    /// Creates a `Message` from bytes that were received from a substream.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DecodeError> {
        Self::try_from(bytes)
    }
}

impl<T> std::fmt::Debug for Message<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let mut b = fmt.debug_struct("Message");
        if !self.want.is_empty() {
            b.field("want", &self.want.iter().map(|k| k.to_string()).collect::<Vec<_>>());
        }
        if !self.have.is_empty() {
            b.field("have", &self.have.iter().map(|k| k.to_string()).collect::<Vec<_>>());
        }
        if !self.cancel.is_empty() {
            b.field("cancel", &self.cancel.iter().map(|x| x.to_string()).collect::<Vec<_>>());
        }
        if !self.blocks.is_empty() {
            b.field(
                "block",
                &self.blocks.iter().map(|x| x.cid().to_string()).collect::<Vec<_>>(),
            );
        }
        if !self.block_presences.is_empty() {
            b.field(
                "block_presences",
                &self
                    .block_presences
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect::<Vec<_>>(),
            );
        }
        if self.full {
            b.field("full", &self.full);
        }
        b.finish()
    }
}

#[derive(Clone, Debug)]
pub enum WantType {
    /// the peer wants us to send the block immediately if we have it
    Block = 0,
    /// the peer just wants to know if we have it
    Have = 1,
}

impl From<WantType> for i32 {
    fn from(value: WantType) -> Self {
        match value {
            WantType::Block => 0,
            WantType::Have => 1,
        }
    }
}

impl TryFrom<i32> for WantType {
    type Error = &'static str;
    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(WantType::Block),
            1 => Ok(WantType::Have),
            _ => Err("invalid enum variant"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{quickcheck, Arbitrary, Gen};
    use std::convert::TryFrom;

    impl Arbitrary for Message<O> {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let want: Vec<Cid> = Arbitrary::arbitrary(g);
            let have: Vec<Cid> = Arbitrary::arbitrary(g);
            let block_presences: Vec<(Cid, bool)> = Arbitrary::arbitrary(g);
            let cancel: Vec<Cid> = Arbitrary::arbitrary(g);
            let blocks: Vec<Block> = Arbitrary::arbitrary(g);
            Self {
                want: want.into_iter().collect(),
                have: have.into_iter().collect(),
                block_presences: block_presences.into_iter().collect(),
                cancel: cancel.into_iter().collect(),
                blocks: blocks.into_iter().collect(),
                full: Arbitrary::arbitrary(g),
                _phantom_data: PhantomData,
            }
        }
    }

    quickcheck! {

        fn cid_bytes_roundtrip(expected: Cid) -> bool {
            let bytes = expected.to_bytes();
            if let Ok(actual) = Cid::try_from(bytes) {
                actual == expected
            } else {
                false
            }
        }

        fn message_bytes_roundtrip(expected: Message<O>) -> bool {
            let bytes = expected.to_bytes();
            if let Ok(actual) = Message::from_bytes(&bytes).map(|x| x.into_output()) {
                actual == expected
            } else {
                false
            }
        }
    }
}
