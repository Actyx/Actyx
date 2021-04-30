use std::collections::BTreeSet;

use actyxos_sdk::{Event, Payload, StreamId};
use derive_more::Display;
use futures::future::BoxFuture;
use futures::stream::BoxStream;
use trees::StreamHeartBeat;

mod backward;
pub mod common;
mod forward;
mod unordered;

pub use common::{EventOrHeartbeat, EventSelection, StreamEventSelection};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Display)]
pub enum ConsumerAccessError {
    #[display(fmt = "Invalid stream back with unbounded set of sources {:?}.", _0)]
    UnboundedStreamBack(EventSelection),
    #[display(fmt = "Cannot stream {} since it is not known.", _0)]
    UnknownStream(StreamId),
}

impl std::error::Error for ConsumerAccessError {}

pub type EventStreamOrError = BoxFuture<'static, Result<BoxStream<'static, Event<Payload>>, ConsumerAccessError>>;

pub type EventOrHeartbeatStreamOrError =
    BoxFuture<'static, Result<BoxStream<'static, EventOrHeartbeat>, ConsumerAccessError>>;

pub trait EventStoreConsumerAccess: Clone + Sized + Sync + Send + 'static {
    /// Returns all stream ids for the local node.
    // Once we have the ability to dynamically create new streams, this should
    // become a `BoxStream`
    fn local_stream_ids(&self) -> BTreeSet<StreamId>;

    /// A stream of all known streams, including future ones (i.e. it never
    /// completes) and without duplicates.
    fn stream_known_streams(&self) -> BoxStream<'static, StreamId>;

    /// Stream events for the given stream id from start to stop; may not finish
    /// if stop is not contained in the currently known “present”. Will be
    /// immediately empty if the given range does not contain events. The events
    /// are streamed in strictly monotonic ascending Offset order.
    ///
    /// **Important note:** This stream is already filtered with the StreamEventSelection,
    /// so it will contain gaps. To account for this, the stream must contain a heartbeat
    /// matching the last event whenever the replay reaches “present” and the last event
    /// was not emitted (although it may emit more heartbeats than that, e.g. whenever an
    /// event is filtered out).
    ///
    /// `from` should be less than `to` for the stream to be non-empty, the stream
    /// starts with the event right after `from` and ends with the event at `to`.
    // per stream_id
    fn stream_forward(&self, events: StreamEventSelection, must_exist: bool) -> EventOrHeartbeatStreamOrError;

    /// Stream events for the given stream id between start and stop in reverse;
    /// returns error if stop is not contained in the currently known “present”.
    /// Will be immediately empty if the given range does not contain events. The
    /// events are streamed in strictly monotonic descending Offset order.
    ///
    /// **Important note:** This stream is already filtered with the NodeEventSelection,
    /// so it will contain gaps.
    ///
    /// `from` should be less than `to` for the stream to be non-empty, so the range
    /// “from–to” is delivered “backwards” by this function (it starts with the `to`
    /// event and move backwards up to but not including the `from` event).
    // per stream_id
    fn stream_backward(&self, events: StreamEventSelection) -> EventStreamOrError;

    /// Stream heartbeats as they become known to the store. Events being added
    /// to the store for a given stream DO NOT constitute heartbeats, they are
    /// delivered via their respective streams. This stream delivers heartbeats
    /// that advance the lower Lamport bound in the absence of events.
    fn stream_last_seen(&self, stream: StreamId) -> BoxStream<'static, StreamHeartBeat>;

    //////////////////////////////////////////////////////
    // Given the above, the methods below are provided: //
    //////////////////////////////////////////////////////

    /// Merge streams from multiple streams to satisfy the EventSelection, keeping
    /// no particular order between the individual substreams’ streams (but keeping order
    /// within each stream’s stream).
    fn stream_events_source_ordered(&self, events: EventSelection) -> EventStreamOrError {
        unordered::stream(self, events)
    }

    /// Merge streams from multiple streams to satisfy the EventSelection, delivering
    /// events strictly in increasing event key order (based on Lamport timestamp and
    /// StreamId). The stream will not advance while not receiving updates from one
    /// of the included streams, where heartbeats are used to make progress while no
    /// real events are being emitted; if the EventSelection only contains events from
    /// the past, the stream will terminate in a timely fashion.
    fn stream_events_forward(&self, events: EventSelection) -> EventStreamOrError {
        forward::stream(self, events)
    }

    /// Merge streams from multiple streams to satisfy the EventSelection, delivering
    /// events strictly in decreasing event key order. This function should only be
    /// used with `to_inclusive` being within the currently known “present”, otherwise
    /// an Error will be returned.
    fn stream_events_backward(&self, events: EventSelection) -> EventStreamOrError {
        backward::stream(self, events)
    }
}
