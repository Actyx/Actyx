use super::common::{stop_when_streams_exhausted, EventOrHeartbeat, EventOrStop};
use super::{EventSelection, EventStoreConsumerAccess, EventStreamOrError};
use ax_futures_util::prelude::*;
use futures::future::{ready, FutureExt};
use futures::stream::{self, StreamExt};

pub fn stream(store: &impl EventStoreConsumerAccess, events: EventSelection) -> EventStreamOrError {
    let local_stream_ids = store.local_stream_ids();
    let maybe_limited_streams = events.get_bounded_nonempty_streams(&local_stream_ids);

    let subs = events.tag_subscriptions.clone();
    // need our own clone of the store (lives 'static thanks to the trait’s bound)
    let store = store.clone();

    if let Some(explicit_streams) = maybe_limited_streams {
        // if the set of streams is bounded, terminate the stream once all have been delivered
        ready(Ok(stop_when_streams_exhausted(
            explicit_streams.clone(),
            store
                .stream_known_streams()
                // We know exactly which streams we're interested in, so we can
                // filter as early as possible
                .filter(move |stream| ready(explicit_streams.contains(stream)))
                .map(move |stream_id| {
                    let events = events.for_stream(stream_id, local_stream_ids.contains(&stream_id));
                    store
                        .stream_forward(events, false)
                        .map(|res| res.unwrap()) // fine since we know that the stream exists
                        .flatten_stream()
                        .filter_map(|x| ready(x.into_event()))
                        .chain(stream::iter(vec![EventOrStop::Stop(stream_id)]))
                })
                .merge_unordered(),
        )
        .boxed()))
        .boxed()
    } else {
        ready(Ok(store
            .stream_known_streams()
            .filter(move |stream| ready(!subs.only_local() || local_stream_ids.contains(stream)))
            .map(move |stream_id| {
                let events = events.for_stream(stream_id, store.local_stream_ids().contains(&stream_id));
                store
                    .stream_forward(events, false)
                    .map(|res| res.unwrap()) // fine since we know that the stream exists
                    .flatten_stream()
                    .filter_map(|x| {
                        if let EventOrHeartbeat::Event(e) = x {
                            ready(Some(e))
                        } else {
                            ready(None)
                        }
                    })
            })
            .merge_unordered()
            .boxed()))
        .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::tests::*;
    use actyxos_sdk::{LamportTimestamp, Offset, OffsetOrMin};
    use ax_futures_util::stream::Drainer;
    use num_traits::Bounded;
    use pretty_assertions::assert_eq;
    use trees::{OffsetMapOrMax, TagSubscriptions};

    #[tokio::test]
    async fn should_deliver_requested_events_with_wildcard_subscription() {
        let store = TestEventStore::new();
        let events = EventSelection::create(
            "FROM 'upper:A' & 'lower:a'",
            &[(test_stream(4), OffsetOrMin::MIN, OffsetOrMin::MAX)],
        )
        .expect("cannot construct selection");
        let mut iter = Drainer::new(stream(&store, events.clone()).await.unwrap());

        let expected = store
            .known_events(test_stream(4), 0, None)
            .into_iter()
            .filter(move |ev| events.matches(ev))
            .collect::<Vec<_>>();
        // the local pool makes this deterministically sorted
        assert_eq!(iter.next(), Some(expected));

        let ev = mk_test_event(
            test_stream(4),
            store.top_offset() + 1,
            LamportTimestamp::from((store.top_offset() - Offset::ZERO) as u64 + 1),
        );
        store.send(Input::Events(vec![ev.clone()]));
        assert_eq!(iter.next(), Some(vec![ev]));
    }

    #[tokio::test]
    async fn should_deliver_and_end_finite_stream() {
        let store = TestEventStore::new();
        let top_offset = store.top_offset();
        let top: OffsetOrMin = top_offset.into();
        let events = EventSelection::create(
            "FROM 'upper:A' & 'lower:a'",
            &[
                (test_stream(1), OffsetOrMin::MIN, top),
                (test_stream(2), OffsetOrMin::mk_test(10), top),
                (test_stream(5), OffsetOrMin::mk_test(50), top),
                (test_stream(7), (top_offset + 10).into(), top),
            ],
        )
        .unwrap();
        let mut iter = Drainer::new(stream(&store, events.clone()).await.unwrap());

        let mut expected = vec![
            store.known_events(test_stream(1), 0, None),
            store.known_events(test_stream(2), 10, None),
            store.known_events(test_stream(5), 0, None),
        ]
        .into_iter()
        .flatten()
        .filter(|ev| events.matches(&ev))
        .map(LamportOrdering)
        .collect::<Vec<_>>();
        expected.sort();

        let res = iter.next().unwrap();
        let mut res = res.into_iter().map(LamportOrdering).collect::<Vec<_>>();
        res.sort();

        println!("res.len {}, expected.len {}", res.len(), expected.len());

        assert_eq!(res, expected);
        assert_eq!(iter.next(), None);
    }

    #[tokio::test]
    async fn should_deliver_requested_events() {
        let store = TestEventStore::new();
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
            .map(LamportOrdering)
            .collect::<Vec<_>>();
        expected.sort();
        let mut result = iter
            .next()
            .expect("no result sent")
            .into_iter()
            .map(LamportOrdering)
            .collect::<Vec<_>>();
        let other = result.clone();
        result.sort();
        assert_eq!(result, expected);
        assert_eq!(iter.next(), None);

        // check that per-stream streams remain ordered
        let s1 = other
            .iter()
            .cloned()
            .filter(|LamportOrdering(ev)| ev.key.stream == test_stream(1))
            .collect::<Vec<_>>();
        let mut s1s = s1.clone();
        s1s.sort();
        assert_eq!(s1, s1s);

        let s2 = other
            .iter()
            .cloned()
            .filter(|LamportOrdering(ev)| ev.key.stream == test_stream(2))
            .collect::<Vec<_>>();
        let mut s2s = s2.clone();
        s2s.sort();
        assert_eq!(s2, s2s);
    }

    #[tokio::test]
    async fn empty_event_selection_should_mean_no_events() {
        let store = TestEventStore::new();
        let events = EventSelection::upto(TagSubscriptions::empty(), OffsetMapOrMax::min_value());
        let mut iter = Drainer::new(stream(&store, events).await.unwrap());

        // stream must terminate immediately
        assert_eq!(iter.next(), None);
    }
}
