#![deny(clippy::future_not_send)]
#[cfg(any(test, feature = "arb"))]
mod arb;
pub mod axtrees;
mod header;
pub mod offsetmap_or_default;
pub mod subscription;
pub mod tag_index;
#[cfg(test)]
mod tests;

pub use self::header::Header as AxTreeHeader;
pub use self::offsetmap_or_default::*;
pub use self::subscription::*;
pub use self::tag_index::*;

use actyxos_sdk::{Event, LamportTimestamp, Offset, StreamId};
use std::cmp::Ordering;

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
