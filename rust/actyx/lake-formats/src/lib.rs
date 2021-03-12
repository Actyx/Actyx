#[cfg(any(test, feature = "arb"))]
mod arb;
pub mod axtrees;
mod bearer_token;
pub mod block;
pub mod cons_node;
pub mod monitoring;
pub mod offsetmap_or_default;
pub mod pubsub;
pub mod subscription;
pub mod tag_index;
#[cfg(test)]
mod tests;
pub mod wrapping_subscriber;

pub use self::block::*;
pub use self::cons_node::*;
pub use self::monitoring::*;
pub use self::offsetmap_or_default::*;
pub use self::pubsub::*;
pub use self::subscription::*;
pub use self::tag_index::*;
pub use bearer_token::BearerToken;

use actyxos_sdk::{
    event::{self, FishName, Semantics, SourceId, StreamInfo},
    fish_name, semantics,
    tagged::{self, Event, StreamId, TagSet},
    LamportTimestamp, Offset, OffsetOrMin, Payload, TimeStamp,
};
use anyhow::{anyhow, Context, Result};
use serde::{ser::Serializer, Deserialize, Deserializer, Serialize};
use std::{
    cmp::Ordering,
    convert::{TryFrom, TryInto},
    fmt::Debug,
    str::{self, FromStr},
    sync::Arc,
};
use tagged::EventKey;
use tracing::{error, warn};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IpfsEnvelope {
    pub semantics: Semantics,
    pub name: FishName,

    // Legacy compatibility: No value for field 'tags' means no tags.
    #[serde(default, skip_serializing_if = "TagSet::is_empty")]
    pub tags: TagSet,

    pub timestamp: TimeStamp,
    #[serde(rename = "psn")]
    pub offset: Offset,
    pub payload: Payload,
    pub lamport: LamportTimestamp,
}

// QUESTION: does this struct have a purpose besides modeling the persistent
// data format? If it does not, would it be possible to unify with IpfsEnvelopeWithSourceId
// dropping the source_id during serialization, and adding the source_id in the deserializer?
// It seems like saving a small number of bytes in a large structure causes a lot of complexity.

impl IpfsEnvelope {
    pub fn with_offset(mut self, offset: Offset) -> Self {
        self.offset = offset;
        self
    }

    pub fn rough_size(&self) -> usize {
        let tags_size: usize = if self.tags.is_empty() {
            0
        } else {
            self.tags.iter().map(|x| x.len()).sum::<usize>() + 24
        };

        tags_size +
        self.semantics.len() + 24 +
        self.name.len() + 24 +
        8 + // timestamp
        8 + // offset
        16 + // lamport
        self.payload.rough_size() + 8
    }

    pub fn with_source(self, source_id: SourceId) -> IpfsEnvelopeWithSourceId {
        IpfsEnvelopeWithSourceId {
            semantics: self.semantics,
            name: self.name,
            tags: self.tags,
            timestamp: self.timestamp,
            offset: self.offset,
            lamport: self.lamport,
            payload: self.payload,
            source_id,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IpfsEnvelopeWithSourceId {
    pub semantics: Semantics,
    pub name: FishName,

    // Legacy compatibility: No value for field 'tags' means no tags.
    #[serde(default, skip_serializing_if = "TagSet::is_empty")]
    pub tags: TagSet,

    pub timestamp: TimeStamp,
    #[serde(rename = "psn")]
    pub offset: Offset,
    pub source_id: SourceId,
    pub payload: Payload,
    pub lamport: LamportTimestamp,
}

impl IpfsEnvelopeWithSourceId {
    pub fn into_raw(self) -> IpfsEnvelope {
        IpfsEnvelope {
            semantics: self.semantics,
            lamport: self.lamport,
            name: self.name,
            tags: self.tags,
            timestamp: self.timestamp,
            offset: self.offset,
            payload: self.payload,
        }
    }

    pub fn get_key(&self) -> EventKey {
        EventKey {
            lamport: self.lamport,
            stream: self.source_id.into(),
            offset: self.offset,
        }
    }

    pub fn clear_payload(self) -> IpfsEnvelopeWithSourceId {
        IpfsEnvelopeWithSourceId {
            source_id: self.source_id,
            semantics: self.semantics,
            name: self.name,
            tags: self.tags,
            offset: self.offset,
            timestamp: self.timestamp,
            payload: Payload::empty(),
            lamport: self.lamport,
        }
    }
}

impl Into<event::Event<Payload>> for IpfsEnvelopeWithSourceId {
    fn into(self) -> event::Event<Payload> {
        event::Event {
            stream: StreamInfo {
                semantics: self.semantics,
                name: self.name,
                source: self.source_id,
            },
            offset: self.offset,
            lamport: self.lamport,
            timestamp: self.timestamp,
            payload: self.payload,
        }
    }
}

impl Into<Event<Payload>> for IpfsEnvelopeWithSourceId {
    fn into(self) -> Event<Payload> {
        Event {
            key: self.get_key(),
            meta: tagged::Metadata {
                timestamp: self.timestamp,
                tags: self.tags,
            },
            payload: self.payload,
        }
    }
}

impl TryFrom<tagged::Event<Payload>> for IpfsEnvelopeWithSourceId {
    type Error = anyhow::Error;
    fn try_from(ev: tagged::Event<Payload>) -> Result<Self> {
        Ok(Self {
            semantics: (&ev.meta.tags).try_into().ok().unwrap_or_else(|| semantics!("_t_")),
            name: (&ev.meta.tags).try_into().ok().unwrap_or_else(|| fish_name!("_t_")),
            tags: ev.meta.tags,
            timestamp: ev.meta.timestamp,
            offset: ev.key.offset,
            source_id: ev.key.stream.source_id().context("converting into v1 event")?,
            payload: ev.payload,
            lamport: ev.key.lamport,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::rc_buffer)]
pub struct EnvelopeList(Arc<Vec<IpfsEnvelope>>);

impl Serialize for EnvelopeList {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_ref().serialize(serializer)
    }
}

const EMPTY_ENVELOPE_LIST: &[IpfsEnvelope] = &[];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OffsetNotInBlock {
    // Offset is lower than all offsets in the block
    Lower,
    // Offset is higher than all offsets in the block
    Higher,
    // Offset was apparently skipped (higher and lower offsets are known to exist)
    Skipped,
}

// Location of an offset within an envelope list (block)
#[derive(Clone, Copy, Debug, PartialEq)]
enum PsnLocation {
    // Offset is immediately before the wrapped index in the list of envelopes
    // (This value is more useful to us than the index, because we want to slice with it,
    // and slice semantics are exactly one off compared to our event selection semantics:
    // While rust slices include `from` and exclude `to`, we EXclude `from` and INclude `to`.)
    ImmediatelyBefore(usize),
    NotInBlock(OffsetNotInBlock),
}

impl From<OffsetNotInBlock> for PsnLocation {
    fn from(absent: OffsetNotInBlock) -> Self {
        PsnLocation::NotInBlock(absent)
    }
}

impl EnvelopeList {
    pub fn new_unwrap(elements: Vec<IpfsEnvelope>) -> Self {
        EnvelopeList::new(elements).expect("at least one event")
    }
    pub fn new(elements: Vec<IpfsEnvelope>) -> Option<EnvelopeList> {
        if !elements.is_empty() {
            Some(EnvelopeList(Arc::new(elements)))
        } else {
            None
        }
    }
    pub fn single(envelope: IpfsEnvelope) -> EnvelopeList {
        EnvelopeList(Arc::new(vec![envelope]))
    }
    pub fn min_offset(&self) -> Offset {
        self.0.first().unwrap().offset
    }
    pub fn max_offset(&self) -> Offset {
        self.0.last().unwrap().offset
    }

    pub fn max_lamport(&self) -> LamportTimestamp {
        self.0.last().unwrap().lamport
    }

    pub fn elements(&self) -> &Vec<IpfsEnvelope> {
        &self.0
    }

    fn get_offset_location(&self, offset: OffsetOrMin) -> PsnLocation {
        if offset < self.min_offset() {
            OffsetNotInBlock::Lower.into()
        } else if offset > self.max_offset() {
            OffsetNotInBlock::Higher.into()
        } else if let Some(concrete_offset) = Offset::from_offset_or_min(offset) {
            let i = (concrete_offset - self.min_offset()) as usize;
            // Cheaply check whether we hit the right index.
            match self.0.get(i) {
                Some(x) if x.offset == concrete_offset => PsnLocation::ImmediatelyBefore(i + 1),
                _ => {
                    error!("offset gap in envelope list {:?}", &self);

                    self.elements()
                        .iter()
                        .position(|envelope| envelope.offset == concrete_offset)
                        .map(|idx| PsnLocation::ImmediatelyBefore(idx + 1))
                        .unwrap_or(PsnLocation::NotInBlock(OffsetNotInBlock::Skipped))
                }
            }
        } else {
            panic!(
                "Offset limit {:?} not concrete, but also not higher or lower than block.",
                offset
            )
        }
    }

    pub fn elements_between(
        &self,
        from_exclusive: OffsetOrMin,
        to_inclusive: OffsetOrMin,
    ) -> anyhow::Result<&[IpfsEnvelope]> {
        use PsnLocation::*;

        match (
            self.get_offset_location(from_exclusive),
            self.get_offset_location(to_inclusive),
        ) {
            (ImmediatelyBefore(offset), ImmediatelyBefore(end)) => Ok(&self.0[offset..end]),
            (NotInBlock(OffsetNotInBlock::Lower), ImmediatelyBefore(end)) => Ok(&self.0[..end]),
            (ImmediatelyBefore(offset), NotInBlock(OffsetNotInBlock::Higher)) => Ok(&self.0[offset..]),
            (NotInBlock(OffsetNotInBlock::Lower), NotInBlock(OffsetNotInBlock::Higher)) => Ok(&self.0),
            (NotInBlock(OffsetNotInBlock::Skipped), _) => {
                Err(anyhow!("concrete from-bound was skipped! {}", from_exclusive))
            }
            (_, NotInBlock(OffsetNotInBlock::Skipped)) => {
                Err(anyhow!("concrete to-bound was skipped! {}", to_inclusive))
            }
            (l, h) => {
                warn!("weird offset hits: {:?} > {:?}", l, h);
                Ok(EMPTY_ENVELOPE_LIST)
            }
        }
    }

    pub fn try_read_offset(&self, offset: Offset) -> Result<IpfsEnvelope, OffsetNotInBlock> {
        match self.get_offset_location(offset.into()) {
            PsnLocation::ImmediatelyBefore(offset) => Ok(self.0[offset - 1].clone()),
            PsnLocation::NotInBlock(absent) => Err(absent),
        }
    }

    pub fn elements_at<'a>(&'a self, indices: &'a [usize]) -> impl Iterator<Item = IpfsEnvelope> + 'a {
        indices.iter().map(move |i| self.0[*i].clone())
    }

    pub fn into_vec(self) -> Vec<IpfsEnvelope> {
        // try to get the content of the arc. If somebody else owns it, we have to clone.
        Arc::try_unwrap(self.0).unwrap_or_else(|err| err.as_ref().clone())
    }
    pub fn rough_size(&self) -> usize {
        self.0.iter().map(|x| x.rough_size()).sum()
    }
}

impl<'de> Deserialize<'de> for EnvelopeList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let envelopes = Vec::<IpfsEnvelope>::deserialize(deserializer)?;
        EnvelopeList::new(envelopes)
            .ok_or_else(|| serde::de::Error::custom("envelope list must contain at least one element"))
    }
}

/// Heartbeat from a stream, implying a promise that future events from this source
/// will have Lamport timestamp and Offset greater than the values advertised here.
///
/// The sorting rules are based only on the Lamport timestamp and two heartbeats
/// can only be compared if their stream id and offset are the same.
#[derive(Debug, Clone, Eq)]
pub struct StreamHeartBeat {
    pub stream: StreamId,
    pub lamport: LamportTimestamp,
    pub offset: Offset,
}

impl PartialOrd for StreamHeartBeat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.stream != other.stream || self.offset != other.offset {
            return None;
        }
        self.lamport.partial_cmp(&other.lamport)
    }
}

impl PartialEq for StreamHeartBeat {
    fn eq(&self, other: &Self) -> bool {
        self.lamport == other.lamport && self.offset == other.offset && self.stream == other.stream
    }
}
impl StreamHeartBeat {
    pub fn new(stream: StreamId, lamport: LamportTimestamp, offset: Offset) -> Self {
        StreamHeartBeat {
            stream,
            lamport,
            offset,
        }
    }

    pub fn from_event<T>(ev: &Event<T>) -> Self {
        Self {
            lamport: ev.key.lamport,
            stream: ev.key.stream,
            offset: ev.key.offset,
        }
    }
}

/// Structure for the connectivity checking engine in store-core
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ConnectivityStatus {
    FullyConnected,
    #[serde(rename_all = "camelCase")]
    PartiallyConnected {
        swarm_connectivity_level: u8, // in percent, between 0 and 100
        events_to_read: u64,          // difference between highest and present
        events_to_send: u64,          // our events unread by others
        specials_disconnected: Vec<SourceId>,
    },
    #[serde(rename_all = "camelCase")]
    NotConnected {
        events_to_read: u64,
        events_to_send: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityResponse {
    #[serde(flatten)]
    pub status: ConnectivityStatus, // current connectivity status
    pub in_current_status_for_ms: u64, // since how long in current status
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityRequest {
    pub special: Vec<SourceId>,
    pub hb_hist_delay: u64,
    pub report_every_ms: u64, // how frequently the connectivity service should report, recommended around 10_000
    #[serde(rename = "currentPsnHistoryDelay")]
    pub current_offset_history_delay: u8, // how many report_every_ms spans back we go for the our_offset value? recommended 6 to give 60s
}

#[cfg(test)]
mod tests_v1 {
    use super::*;
    use actyxos_sdk::{fish_name, semantics, source_id};
    use rstest::*;
    use serde_json::json;

    #[test]
    fn parse_sourceid_successful() {
        let json = r#""abcd""#;
        let source: SourceId = serde_json::from_str(json).expect("Should be able to deserialize");
        assert_eq!(source, SourceId::from_str("abcd").unwrap());
        let text: String = serde_json::to_string(&source).expect("Should be able to serialize");
        assert_eq!(text.as_str(), json);
    }

    #[test]
    fn parse_sourceid_unsuccessful() {
        // json is too long, so it no longer fits into a 16 byte u8 array!
        let json = r#""abcdefgh12345678""#;
        let res: Result<SourceId, _> = serde_json::from_str(json);
        let error = res.expect_err("should not be able to deserialize");
        assert_eq!(format!("{}", error), "SourceId was longer than maximum");
    }

    #[test]
    fn envelope_list_deser_invariants() {
        serde_json::from_str::<EnvelopeList>("[]").unwrap_err();
    }

    #[fixture]
    fn envelope_list() -> EnvelopeList {
        let n: u32 = 10;
        let envelopes = (n..n * 2)
            .map(|i| IpfsEnvelope {
                semantics: semantics!("semantics"),
                name: fish_name!("name"),
                tags: TagSet::empty(),
                timestamp: TimeStamp::now(),
                offset: Offset::mk_test(i),
                lamport: LamportTimestamp::new(i as u64),
                payload: Payload::from_json_value(serde_json::Value::Null).unwrap(),
            })
            .collect::<Vec<_>>();
        EnvelopeList::new_unwrap(envelopes)
    }

    #[fixture]
    fn envelope_list_with_gaps() -> EnvelopeList {
        let n: u32 = 10;
        let envelopes = (n..n * 2)
            .map(|i| IpfsEnvelope {
                semantics: semantics!("semantics"),
                name: fish_name!("name"),
                tags: TagSet::empty(),
                timestamp: TimeStamp::now(),
                offset: Offset::mk_test(i * 2),
                lamport: LamportTimestamp::new(i as u64),
                payload: Payload::from_json_value(serde_json::Value::Null).unwrap(),
            })
            .collect::<Vec<_>>();
        EnvelopeList::new_unwrap(envelopes)
    }

    #[rstest]
    fn get_offset_location_should_not_panic_with_gaps(envelope_list_with_gaps: EnvelopeList) {
        assert_eq!(
            PsnLocation::ImmediatelyBefore(6),
            envelope_list_with_gaps.get_offset_location(OffsetOrMin::from(30u32))
        );
    }

    #[rstest]
    fn get_offset_location_should_not_panic_with_completely_missing_offsets(envelope_list_with_gaps: EnvelopeList) {
        assert_eq!(
            PsnLocation::NotInBlock(OffsetNotInBlock::Skipped),
            envelope_list_with_gaps.get_offset_location(OffsetOrMin::from(29u32))
        );
    }

    #[rstest]
    fn elements_between_select_single_element(envelope_list: EnvelopeList) {
        // Assert that `from` is indeed exclusive, while `to` is inclusive, allowing us
        // to select a single element like this:
        let elems = envelope_list
            .elements_between(OffsetOrMin::mk_test(12), OffsetOrMin::mk_test(13))
            .unwrap();

        assert_eq!(elems[0].offset, Offset::mk_test(13));
    }

    #[rstest]
    fn elements_between_concrete_values(envelope_list: EnvelopeList) {
        let assert_count = |(min, max): (u32, u32), expected: usize| {
            let num_elems = envelope_list
                .elements_between(OffsetOrMin::mk_test(min), OffsetOrMin::mk_test(max))
                .unwrap()
                .len();

            assert_eq!(num_elems, expected)
        };

        // All values inside block
        assert_count((15, 17), 2);
        assert_count((16, 17), 1);
        assert_count((11, 18), 7);
        assert_count((10, 19), 9);
        assert_count((15, 15), 0);

        // Min lower
        assert_count((9, 20), 10);
        assert_count((9, 15), 6);
        assert_count((5, 15), 6);
        assert_count((5, 10), 1);

        // Max higher
        assert_count((10, 20), 9);
        assert_count((11, 25), 8);
        assert_count((18, 30), 1);

        // Both outside
        assert_count((9, 20), 10);
        assert_count((5, 25), 10);
        assert_count((2, 33), 10);

        // No values match
        assert_count((19, 30), 0); // min is exclusive
        assert_count((100, 200), 0);
        assert_count((1, 9), 0);
    }

    #[rstest]
    fn elements_between_with_limits(envelope_list: EnvelopeList) {
        let assert_count = |(min, max): (OffsetOrMin, OffsetOrMin), expected: usize| {
            let num_elems = envelope_list.elements_between(min, max).unwrap().len();

            assert_eq!(num_elems, expected)
        };

        assert_count((OffsetOrMin::MIN, OffsetOrMin::MAX), 10);

        assert_count((OffsetOrMin::MIN, OffsetOrMin::MIN), 0);
        assert_count((OffsetOrMin::MAX, OffsetOrMin::MAX), 0);
        assert_count((OffsetOrMin::MAX, OffsetOrMin::MIN), 0);

        assert_count((OffsetOrMin::MIN, OffsetOrMin::mk_test(12)), 3);
        assert_count((OffsetOrMin::MIN, OffsetOrMin::mk_test(20)), 10);

        assert_count((OffsetOrMin::mk_test(15), OffsetOrMin::MAX), 4);
        assert_count((OffsetOrMin::mk_test(19), OffsetOrMin::MAX), 0);
    }

    #[rstest]
    fn elements_with_offset(envelope_list: EnvelopeList) {
        let assert_offsets = |selected: &[u32]| {
            let offset: Vec<_> = selected.iter().copied().map(Offset::mk_test).collect();
            let elems = offset.iter().map(|o| envelope_list.try_read_offset(*o).unwrap());

            let actual: Vec<_> = elems.map(|x| x.offset).collect();
            assert_eq!(actual, offset)
        };

        // Normal cases
        assert_offsets(&[12, 15]);
        assert_offsets(&[12, 13, 14]);

        // Edges
        assert_offsets(&[10, 19]);
        assert_offsets(&[10, 11]);
        assert_offsets(&[18, 19]);

        // Unordered
        assert_offsets(&[14, 11, 19, 10]);

        // Single elem
        assert_offsets(&[10]);
        assert_offsets(&[11]);
        assert_offsets(&[14]);
        assert_offsets(&[17]);
        assert_offsets(&[19]);

        // Empty
        assert_offsets(&[]);
    }

    #[rstest]
    fn elements_with_offset_gaps2(envelope_list_with_gaps: EnvelopeList) {
        let assert_readerr = |selected: u32| {
            let read = envelope_list_with_gaps.try_read_offset(Offset::mk_test(selected));
            assert!(read.is_err());
        };

        assert_readerr(11);
        assert_readerr(23);
        assert_readerr(25);
    }

    #[rstest]
    fn envelope_list_dedup(envelope_list: EnvelopeList) {
        assert!(envelope_list
            .elements()
            .iter()
            .all(|envelope| Arc::strong_count(envelope.semantics.as_arc()) >= 10
                && Arc::strong_count(envelope.name.as_arc()) >= 10));
    }

    #[test]
    #[allow(clippy::eq_op)]
    fn stream_heartbeat_orders_correctly() {
        let src1 = source_id!("1").into();
        let src2 = source_id!("2").into();

        let s1a = StreamHeartBeat {
            stream: src1,
            offset: Offset::mk_test(1),
            lamport: LamportTimestamp::new(1),
        };
        let s1b = StreamHeartBeat {
            stream: src1,
            offset: Offset::mk_test(1),
            lamport: LamportTimestamp::new(5),
        };
        let s1c = StreamHeartBeat {
            stream: src1,
            offset: Offset::mk_test(2),
            lamport: LamportTimestamp::new(5),
        };

        let s2 = StreamHeartBeat {
            stream: src2,
            offset: Offset::mk_test(1),
            lamport: LamportTimestamp::new(1),
        };

        // equal
        assert_eq!(false, s1a > s1a);
        assert_eq!(true, s1a >= s1a);
        assert_eq!(true, s1a == s1a);
        assert_eq!(false, s1a < s1a);
        assert_eq!(true, s1a <= s1a);

        // less than
        assert_eq!(true, s1a < s1b);
        assert_eq!(true, s1a <= s1b);
        assert_eq!(false, s1a == s1b);
        assert_eq!(false, s1a > s1b);
        assert_eq!(false, s1a >= s1b);

        // incomparable
        assert_eq!(false, s1a < s1c);
        assert_eq!(false, s1a <= s1c);
        assert_eq!(false, s1a == s1c);
        assert_eq!(false, s1a > s1c);
        assert_eq!(false, s1a >= s1c);

        // incomparable
        assert_eq!(false, s1a < s2);
        assert_eq!(false, s1a <= s2);
        assert_eq!(false, s1a == s2);
        assert_eq!(false, s1a > s2);
        assert_eq!(false, s1a >= s2);
    }

    #[test]
    fn connectivity_request_compat() {
        let wire_data = json! {
        {
            "special": ["a", "b"],
            "hbHistDelay": 1234,
            "reportEveryMs": 100,
            "currentPsnHistoryDelay": 100,
        }};
        let req: ConnectivityRequest = serde_json::from_value(wire_data.clone()).unwrap();
        let wire_data_2 = serde_json::to_value(req).unwrap();
        assert_eq!(wire_data, wire_data_2);
    }
}
