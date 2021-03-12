use super::{ConsumerAccessError, EventOrHeartbeat, EventSelection, EventStoreConsumerAccess, EventStreamOrError};
use actyxos_sdk::{
    event::{LamportTimestamp, SourceId},
    tagged::{Event, StreamId},
    Payload,
};
use ax_futures_util::{prelude::*, stream::MergeOrdered};
use futures::future::{ready, try_join_all, BoxFuture, Future, FutureExt, TryFutureExt};
use futures::stream::{self, BoxStream, StreamExt};
use lake_formats::StreamHeartBeat;
use std::{cmp::Ordering, collections::BTreeSet, str::FromStr};

#[derive(Debug, Clone)]
enum EnvelopeOrTick {
    Envelope(Event<Payload>),
    Present(StreamHeartBeat),
    Tick(StreamHeartBeat),
    Stop,
}

impl EnvelopeOrTick {
    pub fn from_store(ev: EventOrHeartbeat) -> Self {
        match ev {
            EventOrHeartbeat::Event(e) => EnvelopeOrTick::Envelope(e),
            EventOrHeartbeat::Heartbeat(hb) => EnvelopeOrTick::Present(hb),
        }
    }
    pub fn is_stop(&self) -> bool {
        matches!(self, EnvelopeOrTick::Stop)
    }
    pub fn to_lamport(&self) -> LamportTimestamp {
        match self {
            EnvelopeOrTick::Envelope(env) => env.key.lamport,
            EnvelopeOrTick::Tick(tick) => tick.lamport,
            EnvelopeOrTick::Present(tick) => tick.lamport,
            EnvelopeOrTick::Stop => LamportTimestamp::new(0),
        }
    }
    pub fn to_stream_id(&self) -> StreamId {
        match self {
            EnvelopeOrTick::Envelope(env) => env.key.stream,
            EnvelopeOrTick::Tick(tick) => tick.stream,
            EnvelopeOrTick::Present(tick) => tick.stream,
            EnvelopeOrTick::Stop => SourceId::from_str("").unwrap().into(),
        }
    }
}

impl Ord for EnvelopeOrTick {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.to_lamport(), self.to_stream_id()).cmp(&(other.to_lamport(), other.to_stream_id()))
    }
}

impl PartialOrd for EnvelopeOrTick {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

impl PartialEq for EnvelopeOrTick {
    fn eq(&self, other: &Self) -> bool {
        self.to_lamport().eq(&other.to_lamport()) && self.to_stream_id().eq(&other.to_stream_id())
    }
}

impl Eq for EnvelopeOrTick {}

fn get_stream_for_stream_id(
    store: &impl EventStoreConsumerAccess,
    events: EventSelection,
    must_exist: bool,
) -> impl FnMut(StreamId) -> BoxFuture<'static, Result<BoxStream<'static, EnvelopeOrTick>, ConsumerAccessError>> {
    let store = store.clone();
    let local_stream_ids = store.local_stream_ids();
    move |stream_id| {
        let events = events.for_stream(stream_id, local_stream_ids.contains(&stream_id));
        let store2 = store.clone();
        store
            .stream_forward(events, must_exist)
            .map_ok(move |stream| {
                // merge events and heartbeats from this stream
                stream::select(
                    stream
                        .map(EnvelopeOrTick::from_store)
                        // inject stop token when done
                        .chain(stream::iter(vec![EnvelopeOrTick::Stop])),
                    store2.stream_last_seen(stream_id).map(EnvelopeOrTick::Tick),
                )
                // .inspect(move |x| println!("{} getting {:?}", source, x))
                // need to stop when the actual event stream stops, last_seen keeps running
                .take_while(move |x| ready(!x.is_stop()))
                // now make sure that heartbeats are kept back until all their referenced
                // events have been delivered
                .map(hold_back_heartbeats_while_waiting_for_corresponding_events())
                .flatten()
                // .inspect(move |x| println!("{} emitting {:?}", stream, x))
                .boxed()
            })
            .boxed()
    }
}

fn hold_back_heartbeats_while_waiting_for_corresponding_events(
) -> impl FnMut(EnvelopeOrTick) -> BoxStream<'static, EnvelopeOrTick> {
    let mut last_heartbeat: Option<StreamHeartBeat> = None;
    let mut last_event: Option<StreamHeartBeat> = None;
    move |eot| {
        let mut res = Vec::<EnvelopeOrTick>::with_capacity(2);
        match eot {
            EnvelopeOrTick::Envelope(env) => {
                last_event = Some(StreamHeartBeat::from_event(&env));
                res.push(EnvelopeOrTick::Envelope(env));
            }
            // This is the store telling us that the subscribed event stream for this stream
            // has caught up with the “present”, we get this to account for the possibility
            // that the most recent event may be discarded by the active event selection.
            // We need to treat it like an event regarding the advancement of stream progress
            // including unblocking potentially waiting heartbeats.
            EnvelopeOrTick::Present(tick) => {
                last_event = Some(tick.clone());
                res.push(EnvelopeOrTick::Present(tick));
            }
            EnvelopeOrTick::Tick(tick) => {
                if last_heartbeat
                    .as_ref()
                    // always keep the newest heartbeat by Lamport, regardless of offset
                    .map(|hb| hb.lamport < tick.lamport)
                    .unwrap_or(true)
                {
                    last_heartbeat = Some(tick);
                }
            }
            _ => (),
        }
        // None < Some(...), heartbeats with different offset are incomparable
        if last_event.is_some() && last_heartbeat > last_event {
            if let Some(hb) = last_heartbeat.take() {
                res.push(EnvelopeOrTick::Tick(hb));
            }
        }
        stream::iter(res).boxed()
    }
}

fn keep_only_envelopes(eot: EnvelopeOrTick) -> impl Future<Output = Option<Event<Payload>>> {
    match eot {
        EnvelopeOrTick::Envelope(env) => ready(Some(env)),
        _ => ready(None),
    }
}

pub fn stream(store: &impl EventStoreConsumerAccess, events: EventSelection) -> EventStreamOrError {
    let local_stream_ids = store.local_stream_ids();
    let mk_stream = get_stream_for_stream_id(store, events.clone(), true);
    if let Some(expected_streams) = events.get_bounded_nonempty_streams(&local_stream_ids) {
        let initials = expected_streams.iter().copied().map(mk_stream);
        try_join_all(initials)
            .map_ok(|initials| {
                MergeOrdered::new_fixed(initials)
                    // .inspect(|x| println!("emitting {:?}", x))
                    .filter_map(keep_only_envelopes)
                    .boxed()
            })
            .boxed()
    } else {
        let expected_streams = events.get_mentioned_streams(&local_stream_ids).collect::<BTreeSet<_>>();
        let initials = events
            .get_mentioned_streams(&local_stream_ids)
            .map(mk_stream)
            .collect::<Vec<_>>();

        let mk_stream = get_stream_for_stream_id(store, events, false);
        let store = store.clone();
        try_join_all(initials)
            .map_ok(move |initials| {
                store
                    .stream_known_streams()
                    // .inspect(|x| println!("proposed {}", x))
                    .filter(move |stream_id| {
                        // ignore already subscribed to streams
                        ready(!expected_streams.contains(stream_id))
                    })
                    // .inspect(|x| println!("accepted {}", x))
                    .then(mk_stream)
                    .map(|res| res.expect("forward stream extra stream cannot fail"))
                    .merge_ordered_with_initials(initials)
                    // .inspect(|x| println!("emitting {:?}", x))
                    .filter_map(keep_only_envelopes)
                    .boxed()
            })
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::EnvelopeOrTick::*;
    use super::*;
    use crate::access::tests::*;
    use actyxos_sdk::{tags, Expression, Offset, OffsetOrMin};
    use ax_futures_util::stream::Drainer;
    use futures::stream::Stream;
    use lake_formats::OffsetMapOrMax;
    use num_traits::Bounded;
    use pretty_assertions::assert_eq;

    async fn drain<T: Clone + Send + 'static>(s: impl Stream<Item = T> + Send + 'static) -> Vec<T> {
        s.collect::<Vec<_>>().await
    }

    #[tokio::test]
    async fn should_hold_back_heartbeats_hb_first() {
        let s = Some(source(0));
        let mut f = hold_back_heartbeats_while_waiting_for_corresponding_events();

        // first heartbeat needs to be stored, waiting for first event
        let h1 = mk_test_heartbeat(s, Offset::mk_test(1), None);
        let f1 = drain(f(Tick(h1))).await;
        assert_eq!(f1, vec![]);

        // first event has equal Lamport and Offset, so heartbeat is dropped
        let ev2 = mk_test_event(s, Offset::mk_test(1), LamportTimestamp::from(1));
        let f2 = drain(f(Envelope(ev2.clone()))).await;
        assert_eq!(f2, vec![Envelope(ev2)]);

        // second heartbeat is in the future of second event
        let h3 = mk_test_heartbeat(s, Offset::mk_test(2), Some(LamportTimestamp::new(5)));
        let f3 = drain(f(Tick(h3.clone()))).await;
        assert_eq!(f3, vec![]);

        // second event unlocks previous heartbeat
        let ev4 = mk_test_event(s, Offset::mk_test(2), LamportTimestamp::from(2));
        let f4 = drain(f(Envelope(ev4.clone()))).await;
        assert_eq!(f4, vec![Envelope(ev4), Tick(h3)]);

        // third heartbeat is in future of third event
        let h3 = mk_test_heartbeat(s, Offset::mk_test(3), Some(LamportTimestamp::new(8)));
        let f3 = drain(f(Tick(h3.clone()))).await;
        assert_eq!(f3, vec![]);

        // third event is filtered out but needs to be represented
        let ev4 = mk_test_event(s, Offset::mk_test(3), LamportTimestamp::from(3));
        let h4 = StreamHeartBeat::from_event(&ev4);
        let f4 = drain(f(Present(h4.clone()))).await;
        assert_eq!(f4, vec![Present(h4), Tick(h3)]);
    }

    #[tokio::test]
    async fn should_hold_back_heartbeats_ev_first() {
        let s = Some(source(0));
        let mut f = hold_back_heartbeats_while_waiting_for_corresponding_events();

        // first event has Offset::mk_test(1) and Lamport(1)
        let ev1 = mk_test_event(s, Offset::mk_test(1), LamportTimestamp::new(1));
        let f1 = drain(f(Envelope(ev1.clone()))).await;
        assert_eq!(f1, vec![Envelope(ev1)]);

        // heartbeat just improves on that
        let h2 = mk_test_heartbeat(s, Offset::mk_test(1), Some(LamportTimestamp::new(5)));
        let f2 = drain(f(Tick(h2.clone()))).await;
        assert_eq!(f2, vec![Tick(h2)]);
    }

    #[tokio::test]
    async fn should_deliver_bounded_stream() {
        let store = TestEventStore::new();
        let events = EventSelection::create(
            "('upper:A' & 'lower:a') | 'upper:B'",
            &[
                (source(1), OffsetOrMin::mk_test(40), OffsetOrMin::mk_test(70)),
                (source(2), OffsetOrMin::MIN, OffsetOrMin::mk_test(62)),
            ],
        )
        .expect("cannot construct selection");
        let mut iter = Drainer::new(stream(&store, events.clone()).await.unwrap());

        println!(
            "expected sources {:?}",
            events
                .get_mentioned_streams(&store.local_stream_ids())
                .collect::<Vec<_>>()
        );
        let mut expected = store
            .known_events(source(1), 0, None)
            .into_iter()
            .chain(store.known_events(source(2), 0, None))
            .filter(move |ev| events.matches(ev))
            .map(LamportOrdering)
            .collect::<Vec<_>>();
        expected.sort();

        let res = iter
            .next()
            .map(|x| x.into_iter().map(LamportOrdering).collect::<Vec<_>>())
            .unwrap();
        assert_eq!(res, expected);
        assert_eq!(iter.next(), None);
    }

    #[tokio::test]
    async fn should_deliver_live_stream() {
        let store = TestEventStore::new();
        let events = EventSelection::create(
            "('upper:A' & 'lower:a') | 'upper:B'",
            &[
                (source(0), OffsetOrMin::mk_test(4), OffsetOrMin::mk_test(39)),
                (source(1), OffsetOrMin::mk_test(40), OffsetOrMin::MAX),
                (source(2), OffsetOrMin::MIN, OffsetOrMin::MAX),
            ],
        )
        .expect("cannot construct selection");
        let mut iter = Drainer::new(stream(&store, events.clone()).await.unwrap());

        let mut expected = store
            .known_events(source(0), 0, None)
            .into_iter()
            .chain(store.known_events(source(1), 0, None))
            .chain(store.known_events(source(2), 0, None))
            .filter(move |ev| events.matches(ev))
            .map(LamportOrdering)
            .collect::<Vec<_>>();
        expected.sort();

        let res = iter
            .next()
            .map(|x| x.into_iter().map(LamportOrdering).collect::<Vec<_>>())
            .unwrap();
        assert_eq!(res, expected);

        // At this point source0 is finished, source1 has notified the merge of top_offset,
        // source2 has done the same, so everything up to that point is delivered.
        // Now we want to test that a heartbeat from source1 lets the live stream progress:
        // fabricate one and send an event for source2 that should be emitted.
        let top = store.top_offset();
        let top_lamport = LamportTimestamp::from((top - Offset::ZERO) as u64);

        // at first the heartbeat is blocked
        let hb = mk_test_heartbeat(
            Some(source(1)),
            top + 1,
            Some(LamportTimestamp::new((top - Offset::ZERO) as u64 + 5)),
        );
        store.send(Input::Heartbeat(hb));
        // unblock heartbeat; the event is also blocked as there is nothing from source2
        let ev1 = mk_test_event(Some(source(1)), top + 1, top_lamport + 1);
        println!("ev1 {:?}", ev1);
        store.send(Input::Events(vec![ev1.clone()]));
        assert_eq!(iter.next(), Some(vec![]));

        // now inject event from source2 that should unblock event from source 1
        // and be unblocked by source1 heartbeat
        let mut ev2 = mk_test_event(Some(source(2)), top + 1, top_lamport + 1);
        ev2.meta.tags = tags!("upper:B");
        store.send(Input::Events(vec![ev2.clone()]));
        assert_eq!(iter.next(), Some(vec![ev1, ev2]));

        // now check that source2 can still make progress while the source1 heartbeat
        // remains large enough
        let mut ev2 = (2..8)
            .map(|i| mk_test_event(Some(source(2)), top + i, top_lamport + i as u64))
            .collect::<Vec<_>>();
        for ev in &mut ev2 {
            ev.meta.tags = tags!("upper:B");
        }
        store.send(Input::Events(ev2.clone()));
        assert_eq!(iter.next(), Some(ev2[..3].to_owned()));
    }

    #[tokio::test]
    async fn should_bring_in_new_sources() {
        let store = TestEventStore::new();
        let source_a = test_stream_id(); // SourceId::from_str("sourceA").unwrap();
        let source_b = test_stream_id(); //SourceId::from_str("sourceB").unwrap();

        // only live events from sources 1 & 2 or unknown sources
        let mut from = (0..10).map(|i| (source(i), OffsetOrMin::MAX)).collect::<Vec<_>>();
        from[1].1 = store.top_offset().into();
        from[2].1 = store.top_offset().into();

        let events = EventSelection::new(
            "('upper:A' & 'lower:a') | 'upper:B' | 'upper:C'"
                .parse::<Expression>()
                .unwrap()
                .into(),
            OffsetMapOrMax::from_entries(&*from),
            OffsetMapOrMax::max_value(),
        );
        assert_eq!(events.get_bounded_nonempty_streams(&store.local_stream_ids()), None);
        assert_eq!(
            events
                .get_mentioned_streams(&store.local_stream_ids())
                .collect::<BTreeSet<_>>(),
            [source(1), source(2)].iter().cloned().collect::<BTreeSet<_>>()
        );
        assert_eq!(events.for_stream(source_a, false).from_exclusive, OffsetOrMin::MIN);
        assert_eq!(events.for_stream(source_a, false).to_inclusive, OffsetOrMin::MAX);
        let mut iter = Drainer::new(stream(&store, events).await.unwrap());

        let top_plus = |x: u32| store.top_offset() + x;
        let lamport = |x: u64| LamportTimestamp::new((store.top_offset() - Offset::ZERO) as u64 + x);

        // should wait for source1 and source2 as per the subscription set
        let ev1 = mk_test_event(Some(source(1)), top_plus(1), lamport(1));
        store.send(Input::Events(vec![ev1.clone()]));
        assert_eq!(iter.next(), Some(vec![]));

        // prepare a heartbeat for source2, to be activated when ev2 is sent
        let hb2 = mk_test_heartbeat(Some(source(2)), top_plus(1), Some(lamport(3)));
        store.send(Input::Heartbeat(hb2));
        assert_eq!(iter.next(), Some(vec![]));

        // let’s introduce sourceA without sending an event, yet
        let hb_a = mk_test_heartbeat(Some(source_a), Offset::mk_test(0), Some(lamport(3)));
        store.send(Input::Heartbeat(hb_a));
        assert_eq!(iter.next(), Some(vec![]));

        // should wait for sourceA now
        let mut ev2 = mk_test_event(Some(source(2)), top_plus(1), lamport(1));
        ev2.meta.tags = tags!("upper:B");
        store.send(Input::Events(vec![ev2.clone()]));
        assert_eq!(iter.next(), Some(vec![]));

        // // send for sourceA, unblock source1, sourceA has heartbeat queued
        let ev_a = mk_test_event(Some(source_a), Offset::mk_test(0), lamport(1));
        store.send(Input::Events(vec![ev_a.clone()]));
        assert_eq!(iter.next(), Some(vec![ev1]));

        // send for source1, unblock source2, which will immediately get its heartbeat
        // and unblock sourceA, which will immediately get its heartbeat and unblock source1
        let ev1 = mk_test_event(Some(source(1)), top_plus(2), lamport(2));
        store.send(Input::Events(vec![ev1.clone()]));
        assert_eq!(iter.next(), Some(vec![ev2, ev_a, ev1]));

        // At this point, source1 has nothing, source2 has its heartbeat for lamport(3),
        // sourceA also has its heartbeat for lamport(3). We bring in sourceB with an
        // old event that should be dropped and then with a new event that should be emitted.

        // NOTE: the TestEventStore does not support adding new sources by simply publishing
        // an event, we need to emit a heartbeat and then start at top_plus(1).
        let hb_b = mk_test_heartbeat(Some(source_b), Offset::mk_test(0), None);
        store.send(Input::Heartbeat(hb_b));
        // need to let the stream run so it can set up the subscriptions ...
        assert_eq!(iter.next(), Some(vec![]));

        // now inject old event from new sourceB
        let mut ev_b = mk_test_event(Some(source_b), Offset::mk_test(0), lamport(1));
        ev_b.meta.tags = tags!("upper:C");
        store.send(Input::Events(vec![ev_b]));
        assert_eq!(iter.next(), Some(vec![]));

        // hearbeat for source1 to unblock sourceB (but old will be dropped)
        let hb1 = mk_test_heartbeat(Some(source(1)), top_plus(2), Some(lamport(3)));
        store.send(Input::Heartbeat(hb1));
        assert_eq!(iter.next(), Some(vec![]));

        // and finally a new event from sourceB that is now the oldest
        let mut ev_b = mk_test_event(Some(source_b), Offset::mk_test(1), lamport(2));
        ev_b.meta.tags = tags!("upper:C");
        store.send(Input::Events(vec![ev_b.clone()]));
        assert_eq!(iter.next(), Some(vec![ev_b]));
    }
}
