use super::{ConsumerAccessError, EventSelection, EventStoreConsumerAccess, EventStreamOrError};
use actyxos_sdk::{Event, LamportTimestamp, Payload, StreamId};
use ax_futures_util::stream::MergeOrdered;
use futures::future::{err, try_join_all, BoxFuture, FutureExt, TryFutureExt};
use futures::stream::{BoxStream, StreamExt};
use std::cmp::Ordering;

use tracing::error;

#[derive(Debug, Clone)]
struct Envelope(Event<Payload>);

impl Envelope {
    pub fn to_lamport(&self) -> LamportTimestamp {
        self.0.key.lamport
    }
    pub fn to_stream(&self) -> StreamId {
        self.0.key.stream
    }
}

// CAUTION: this uses reverse ordering throughout!
impl Ord for Envelope {
    fn cmp(&self, other: &Self) -> Ordering {
        (other.to_lamport(), other.to_stream()).cmp(&(self.to_lamport(), self.to_stream()))
    }
}

impl PartialOrd for Envelope {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

impl PartialEq for Envelope {
    fn eq(&self, other: &Self) -> bool {
        self.to_lamport().eq(&other.to_lamport()) && self.to_stream().eq(&other.to_stream())
    }
}

impl Eq for Envelope {}

fn get_stream_for_stream_id(
    store: &impl EventStoreConsumerAccess,
    events: EventSelection,
) -> impl FnMut(StreamId) -> BoxFuture<'static, Result<BoxStream<'static, Envelope>, ConsumerAccessError>> {
    let store = store.clone();
    let local_stream_ids = store.local_stream_ids();
    move |stream_id| {
        let events = events.for_stream(stream_id, local_stream_ids.contains(&stream_id));
        store
            .stream_backward(events)
            .map_ok(|stream| stream.map(Envelope).boxed())
            .boxed()
    }
}

/// Stream events from this event selection backwards. This only works if the
/// event selection corresponds to a fixed known set of streams and their upper
/// offset bounds are all contained in the currently known “present”. Streams
/// may be restricted by the PsnMaps or by not having wildcard subscriptions.
pub fn stream(store: &impl EventStoreConsumerAccess, events: EventSelection) -> EventStreamOrError {
    if let Some(expected_streams) = events.get_bounded_nonempty_streams(&store.local_stream_ids()) {
        let mk_stream = get_stream_for_stream_id(store, events);
        let streams = expected_streams.iter().copied().map(mk_stream);
        try_join_all(streams)
            .map_ok(|streams| MergeOrdered::new_fixed(streams).map(|env| env.0).boxed())
            .boxed()
    } else {
        error!("Cannot stream unbounded event selection backwards: {:?}", events);
        err(ConsumerAccessError::UnboundedStreamBack(events)).boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::tests::*;
    use actyxos_sdk::OffsetOrMin;
    use ax_futures_util::stream::Drainer;
    use pretty_assertions::assert_eq;

    #[tokio::test]
    async fn should_deliver_requested_events() {
        let store = tests::TestEventStore::new();
        let events = EventSelection::create(
            "FROM ('upper:A' & 'lower:a') | 'upper:B'",
            &[
                (test_stream(1), OffsetOrMin::mk_test(40), OffsetOrMin::mk_test(70)),
                (test_stream(2), OffsetOrMin::MIN, OffsetOrMin::mk_test(62)),
            ],
        )
        .expect("cannot construct selection");
        let mut iter = Drainer::new(stream(&store, events.clone()).await.unwrap());

        let mut expected = store
            .known_events(test_stream(1), 0, None)
            .into_iter()
            .chain(store.known_events(test_stream(2), 0, None))
            .filter(move |ev| events.matches(ev))
            .map(tests::LamportOrdering)
            .collect::<Vec<_>>();
        expected.sort();
        expected.reverse();

        let res = iter
            .next()
            .map(|x| x.into_iter().map(tests::LamportOrdering).collect::<Vec<_>>())
            .unwrap();
        assert_eq!(res, expected);
        assert_eq!(iter.next(), None);
    }
}
