use crate::access::{
    EventOrHeartbeat, EventOrHeartbeatStreamOrError, EventStoreConsumerAccess, EventStreamOrError, StreamEventSelection,
};
use ::trees::{StreamHeartBeat, TagSubscriptions};
use actyxos_sdk::{
    language::Expression, tags, Event, EventKey, LamportTimestamp, Metadata, NodeId, Offset, OffsetOrMin, Payload,
    StreamId, Tag, TagSet, Timestamp,
};
use ax_futures_util::{
    prelude::*,
    stream::{variable::Variable, Drainer},
};
use futures::{
    channel::mpsc::{self, UnboundedSender},
    future::{ok, ready, FutureExt},
    stream::{self, BoxStream, StreamExt},
};
use pretty_assertions::assert_eq;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, BinaryHeap},
    sync::{Arc, Mutex},
};
use tracing::{debug, trace};

use quickcheck::Arbitrary;
mod trees;

#[derive(Debug, Clone)]
pub enum Input {
    Events(Vec<Event<Payload>>),
    Heartbeat(StreamHeartBeat),
}

impl Input {
    pub fn to_stream(&self) -> StreamId {
        match self {
            Input::Events(vec) => vec[0].key.stream,
            Input::Heartbeat(hb) => hb.stream,
        }
    }
}

/// Reverse Offset order to get the smallest envelope out of a heap
#[derive(Debug)]
struct PsnOrder(Event<Payload>);
impl Ord for PsnOrder {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.key.offset.cmp(&self.0.key.offset)
    }
}
impl PartialOrd for PsnOrder {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}
impl PartialEq for PsnOrder {
    fn eq(&self, other: &Self) -> bool {
        self.0.key.offset.eq(&other.0.key.offset)
    }
}
impl Eq for PsnOrder {}

type StreamData = (BTreeMap<StreamId, PerStream>, Variable<BTreeSet<StreamId>>);

/// An EventStore that synthesizes event streams for stream0 to stream9,
/// containing events from three semantics A, B, C with names a, b, c each.
/// Each stream has events 0..10. Psns are allocated round-robin to names, then
/// semantics.
///
/// Additional events can be injected by the test procedure, just has live
/// heartbeats. The injected information is NOT stored for future subscribers.
#[derive(Debug, Clone)]
pub struct TestEventStore {
    evs: Vec<Event<Payload>>,
    streams: Arc<Mutex<StreamData>>,
}

#[derive(Debug, Clone)]
struct PerStream {
    event_senders: Vec<UnboundedSender<Vec<Event<Payload>>>>,
    hb_sender: Variable<Option<StreamHeartBeat>>,
}

impl TestEventStore {
    pub fn new() -> TestEventStore {
        let mut evs = Vec::new();
        let mut off = 0u32;
        for stream in known_streams() {
            for semantics in vec!["A", "B", "C"].iter().map(|x| format!("upper:{}", x)) {
                for name in vec!["a", "b", "c"].iter().map(|x| format!("lower:{}", x)) {
                    evs.push(Event {
                        key: EventKey {
                            lamport: LamportTimestamp::new(off.into()),
                            offset: Offset::mk_test(off),
                            stream,
                        },
                        meta: Metadata {
                            timestamp: Timestamp::now(),
                            tags: vec![semantics.clone(), name.clone()]
                                .into_iter()
                                .map(|x| Tag::new(x).unwrap())
                                .collect::<TagSet>(),
                        },
                        payload: Payload::from_json_str(&*format!("{}", off)).unwrap(),
                    });
                    off += 1;
                }
            }
        }
        let stream_set = known_streams().into_iter().collect::<BTreeSet<_>>();
        let streams_snd = Variable::new(stream_set);
        let streams = Arc::new(Mutex::new((BTreeMap::new(), streams_snd)));
        TestEventStore { evs, streams }
    }

    pub fn known_events(&self, stream_id: StreamId, start: usize, stop: Option<usize>) -> Vec<Event<Payload>> {
        (if !is_known_stream(stream_id) {
            &[]
        } else if let Some(stop) = stop {
            &self.evs[start..stop]
        } else {
            &self.evs[start..]
        })
        .iter()
        .cloned()
        .map(|mut ev| {
            ev.key.stream = stream_id;
            ev
        })
        .collect::<Vec<_>>()
    }

    pub fn known_events_rev(&self, stream_id: StreamId, start: usize, stop: Option<usize>) -> Vec<Event<Payload>> {
        let mut res = self.known_events(stream_id, start, stop);
        res.reverse();
        res
    }

    fn stream_entry(&self, stream_id: StreamId, sender: Option<UnboundedSender<Vec<Event<Payload>>>>) -> PerStream {
        let mut streams = self.streams.lock().unwrap();
        let (ref mut map, ref mut snd) = &mut *streams;
        snd.transform_mut(|set| set.insert(stream_id));
        let entry = map.entry(stream_id).or_insert_with(|| PerStream {
            event_senders: Vec::new(),
            hb_sender: Variable::new(None),
        });
        if let Some(snd) = sender {
            entry.event_senders.push(snd);
        }
        entry.clone()
    }

    /// Caveat emptor: sending only works for already known streams, so be sure to send a heartbeat first!
    ///
    /// You may use `store.introduce_stream()` for that.
    pub fn send(&self, input: Input) {
        debug!("sending {:?}", input);
        let entry = self.stream_entry(input.to_stream(), None);
        match input {
            Input::Events(evs) => {
                for mut s in entry.event_senders {
                    s.start_send(evs.clone()).unwrap();
                }
            }
            Input::Heartbeat(hb) => entry.hb_sender.set(Some(hb)),
        }
    }

    pub fn top_offset(&self) -> Offset {
        Offset::mk_test(self.evs.len() as u32 - 1)
    }

    pub fn top_for_stream(&self, stream_id: StreamId) -> OffsetOrMin {
        if is_known_stream(stream_id) {
            self.top_offset().into()
        } else {
            OffsetOrMin::MIN
        }
    }

    fn stream_contiguous_after(&self, offset: OffsetOrMin, stream_id: StreamId) -> BoxStream<'static, Event<Payload>> {
        trace!(
            offset = offset - OffsetOrMin::ZERO,
            // stream = stream.as_str(),
            "starting stream"
        );
        let (snd, rcv) = mpsc::unbounded();
        self.stream_entry(stream_id, Some(snd));
        rcv.map(stream::iter)
            .flatten()
            .inspect(move |e| {
                trace!(
                    offset = offset - OffsetOrMin::ZERO,
                    // stream = stream.to_string(),
                    "received {:?}",
                    e
                )
            })
            .filter(move |ev| ready(ev.key.stream == stream_id))
            .map({
                let mut heap = BinaryHeap::<PsnOrder>::new();
                let mut last_offset = offset;
                move |ev| {
                    heap.push(PsnOrder(ev));
                    let mut res = vec![];
                    loop {
                        let next_offset = last_offset + 1;
                        if heap.peek().map(|h| h.0.key.offset.into()).unwrap_or(last_offset) != next_offset {
                            break;
                        }
                        res.push(heap.pop().unwrap().0);
                        last_offset = next_offset;
                    }
                    stream::iter(res)
                }
            })
            .flatten()
            .boxed()
    }

    fn to_index(&self, v: OffsetOrMin) -> usize {
        ((v - OffsetOrMin::MIN) as usize).min(self.evs.len())
    }
}

/// An ordering wrapper that establishes Lamport timestamp ordering.
#[derive(Debug, Clone)]
pub struct LamportOrdering(pub Event<Payload>);

impl Ord for LamportOrdering {
    fn cmp(&self, other: &LamportOrdering) -> Ordering {
        self.0
            .key
            .lamport
            .cmp(&other.0.key.lamport)
            .then_with(|| self.0.key.stream.cmp(&other.0.key.stream))
    }
}

impl PartialOrd for LamportOrdering {
    fn partial_cmp(&self, other: &LamportOrdering) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for LamportOrdering {
    fn eq(&self, other: &LamportOrdering) -> bool {
        self.0.key.lamport == other.0.key.lamport && self.0.key.stream == other.0.key.stream
    }
}

impl Eq for LamportOrdering {}

pub fn known_streams() -> Vec<StreamId> {
    (0..5)
        .map(|x| std::iter::once(x).chain(0..31).collect::<Vec<u8>>())
        .map(|x| NodeId::from_bytes(&x[..]).unwrap())
        .flat_map(|node_id| (0..2).map(move |i| node_id.stream(i.into())))
        .collect()
}

pub fn is_known_stream(stream_id: StreamId) -> bool {
    known_streams().contains(&stream_id)
}

pub fn test_stream(i: usize) -> StreamId {
    known_streams()[i]
}

impl EventStoreConsumerAccess for TestEventStore {
    fn local_stream_ids(&self) -> BTreeSet<StreamId> {
        BTreeSet::new()
    }

    fn stream_known_streams(&self) -> BoxStream<'static, StreamId> {
        trace!("streaming known streams");
        let mut streams = BTreeSet::new();
        let obs = self.streams.lock().unwrap().1.new_observer();
        obs.map(move |set| {
            let new = &set - &streams;
            streams = set;
            stream::iter(new)
        })
        .flatten()
        .boxed()
    }

    fn stream_forward(&self, events: StreamEventSelection, _must_exist: bool) -> EventOrHeartbeatStreamOrError {
        trace!("stream_forward {:?}", events);
        let StreamEventSelection {
            stream_id,
            from_exclusive,
            to_inclusive,
            tag_subscriptions: subs,
        } = events;

        if from_exclusive >= to_inclusive {
            return ok(stream::empty().boxed()).boxed();
        }

        // array indexing is [_,_) while stream selection is (_,_]
        let start = self.to_index(from_exclusive);
        let stop = self.to_index(to_inclusive);
        let evs = self.known_events(stream_id, start, Some(stop));
        let hb = evs.last().map_or(vec![], |ev| {
            vec![EventOrHeartbeat::Heartbeat(StreamHeartBeat::from_event(ev))]
        });
        let subs = subs;
        // does it reach into live territory?
        if to_inclusive > self.top_for_stream(stream_id) {
            let subs2 = subs.clone();
            ok(stream::iter(
                evs.into_iter()
                    .filter(move |ev| matches(&subs, &ev.meta.tags))
                    .map(EventOrHeartbeat::Event),
            )
            .chain(stream::iter(hb))
            .chain(
                self.stream_contiguous_after(self.top_for_stream(stream_id), stream_id)
                    .skip_while(move |ev| ready(ev.key.offset <= from_exclusive))
                    .filter_map(move |ev| {
                        if matches(&subs2, &ev.meta.tags) {
                            ready(Some(EventOrHeartbeat::Event(ev)))
                        } else {
                            ready(Some(EventOrHeartbeat::Heartbeat(StreamHeartBeat::from_event(&ev))))
                        }
                    })
                    .take_until_condition(move |ev| ready(ev.offset() >= to_inclusive)),
            )
            // .inspect(move |e| trace!(stream = stream.as_str(), "live emitting {:?}", e))
            .boxed())
            .boxed()
        } else {
            ok(stream::iter(evs)
                .filter(move |ev| ready(matches(&subs, &ev.meta.tags)))
                .map(EventOrHeartbeat::Event)
                .chain(stream::iter(hb))
                // .inspect(move |e| trace!(stream = stream.as_str(), "cold emitting {:?}", e))
                .boxed())
            .boxed()
        }
    }

    fn stream_backward(&self, events: StreamEventSelection) -> EventStreamOrError {
        let StreamEventSelection {
            stream_id,
            from_exclusive,
            to_inclusive,
            tag_subscriptions: subs,
        } = events;

        if from_exclusive >= to_inclusive {
            return ok(stream::empty().boxed()).boxed();
        }

        // does it reach into live territory?
        if to_inclusive > self.top_offset() {
            unimplemented!();
        }

        // array indexing is [_,_) while stream selection is (_,_]
        let start = self.to_index(from_exclusive);
        let stop = self.to_index(to_inclusive);
        let mut evs = self.known_events(stream_id, start, Some(stop));
        evs.reverse();
        let subs = subs;
        ok(stream::iter(evs)
            .filter(move |ev| ready(matches(&subs, &ev.meta.tags)))
            .boxed())
        .boxed()
    }

    fn stream_last_seen(&self, stream_id: StreamId) -> BoxStream<'static, StreamHeartBeat> {
        let entry = self.stream_entry(stream_id, None);
        let mut last_lamport = LamportTimestamp::new(0);
        entry
            .hb_sender
            .new_observer()
            .filter_map(ready)
            .filter(move |hb| {
                if hb.lamport > last_lamport {
                    last_lamport = hb.lamport;
                    ready(true)
                } else {
                    ready(false)
                }
            })
            .boxed()
    }
}

fn matches(slf: &[TagSet], other: &TagSet) -> bool {
    slf.iter().any(|x| x.is_subset(other))
}

/// generate a random StreamId — for test purposes only!
/// Note: The StreamNr is always 0
pub fn test_stream_id() -> StreamId {
    let mut gen = quickcheck::Gen::new(42);
    NodeId::arbitrary(&mut gen).stream(0.into())
}

/// Creates a test event with tags!("upper:A", "lower:a")
pub fn mk_test_event(stream_id: StreamId, offset: Offset, lamport: LamportTimestamp) -> Event<Payload> {
    Event {
        key: EventKey {
            lamport,
            offset,
            stream: stream_id,
        },
        meta: Metadata {
            tags: tags!("upper:A", "lower:a"),
            timestamp: Timestamp::new(1324),
        },
        payload: Payload::empty(),
    }
}

pub fn mk_test_heartbeat(stream_id: StreamId, offset: Offset, lamport: Option<LamportTimestamp>) -> StreamHeartBeat {
    StreamHeartBeat {
        stream: stream_id,
        offset,
        lamport: lamport.unwrap_or_else(|| LamportTimestamp::new((offset - Offset::ZERO) as u64)),
    }
}

async fn drainer(e: EventOrHeartbeatStreamOrError) -> impl Iterator<Item = Vec<Event<Payload>>> {
    Drainer::new(e.await.unwrap()).map(|evs| {
        evs.into_iter()
            .filter_map(|ev| match ev {
                EventOrHeartbeat::Event(e) => Some(e),
                _ => None,
            })
            .collect::<Vec<_>>()
    })
}

#[test]
fn test_harness_should_deliver_known_streams() {
    let store = TestEventStore::new();

    let stream = store.stream_known_streams();
    let mut iter = Drainer::new(stream);
    assert_eq!(iter.next(), Some(known_streams()));

    let ev = mk_test_event(test_stream_id(), Offset::ZERO, LamportTimestamp::from(0));
    store.send(Input::Events(vec![ev.clone()]));
    assert_eq!(iter.next(), Some(vec![ev.key.stream]));
    store.send(Input::Events(vec![ev]));
    assert_eq!(iter.next(), Some(vec![]));

    let stream_id = test_stream_id();
    let hb = mk_test_heartbeat(stream_id, Offset::mk_test(0), None);
    store.send(Input::Heartbeat(hb.clone()));
    assert_eq!(iter.next(), Some(vec![stream_id]));
    store.send(Input::Heartbeat(hb));
    assert_eq!(iter.next(), Some(vec![]));
}

#[test]
fn test_harness_should_deliver_last_seen() {
    let store = TestEventStore::new();
    let ev = mk_test_event(test_stream_id(), Offset::ZERO, LamportTimestamp::from(0));

    let stream = store.stream_last_seen(ev.key.stream);
    let mut iter = Drainer::new(stream);
    assert_eq!(iter.next(), Some(vec![]));

    store.send(Input::Events(vec![ev.clone()]));
    assert_eq!(iter.next(), Some(vec![]));

    let hb = mk_test_heartbeat(ev.key.stream, Offset::mk_test(1), None);
    store.send(Input::Heartbeat(hb.clone()));
    assert_eq!(iter.next(), Some(vec![hb.clone()]));
    store.send(Input::Heartbeat(hb));
    assert_eq!(iter.next(), Some(vec![]));
}

#[tokio::test]
async fn test_harness_should_filter_events_forward() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];
    let subs: TagSubscriptions = "FROM 'upper:A' & ('lower:a' | 'lower:b')"
        .parse::<Expression>()
        .unwrap()
        .into();

    let events = StreamEventSelection::new(stream_id, OffsetOrMin::MIN, OffsetOrMin::MAX, subs.into());
    let stream = store.stream_forward(events, true).await.unwrap();
    let mut iter = Drainer::new(stream);

    let evs = store.known_events(stream_id, 0, None);
    let hb = EventOrHeartbeat::Heartbeat(StreamHeartBeat::from_event(evs.last().unwrap()));
    let mut expected = evs
        .into_iter()
        .filter(|ev| {
            [tags!("upper:A", "lower:a"), tags!("upper:A", "lower:b")]
                .iter()
                .any(|t| ev.meta.tags.is_subset(t))
        })
        .map(EventOrHeartbeat::Event)
        .collect::<Vec<_>>();
    expected.push(hb);

    assert_eq!(iter.next(), Some(expected));
}

#[tokio::test]
async fn test_harness_should_filter_events_backward() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];
    let subs: TagSubscriptions = "FROM 'upper:A' & ('lower:a' | 'lower:b')"
        .parse::<Expression>()
        .unwrap()
        .into();

    let events = StreamEventSelection::new(stream_id, OffsetOrMin::MIN, store.top_offset().into(), subs.into());
    let stream = store.stream_backward(events).await.unwrap();
    let mut iter = Drainer::new(stream);

    let mut expected = store.known_events_rev(stream_id, 0, None);
    expected.retain(|ev| {
        [tags!("upper:A", "lower:a"), tags!("upper:A", "lower:b")]
            .iter()
            .any(|t| ev.meta.tags.is_subset(t))
    });

    assert_eq!(iter.next(), Some(expected));
}

#[tokio::test]
async fn test_harness_should_deliver_events_forward_min_max() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];
    let subs: TagSubscriptions = "FROM 'upper:A' & ('lower:a' | 'lower:b')"
        .parse::<Expression>()
        .unwrap()
        .into();

    let events = StreamEventSelection::new(stream_id, OffsetOrMin::MIN, OffsetOrMin::MAX, subs.into());
    let stream = store.stream_forward(events, true).await.unwrap();
    let mut iter = Drainer::new(stream);

    let evs = store.known_events(stream_id, 0, None);
    let hb = EventOrHeartbeat::Heartbeat(StreamHeartBeat::from_event(evs.last().unwrap()));
    let mut expected = evs
        .into_iter()
        .filter(|ev| {
            [tags!("upper:A", "lower:a"), tags!("upper:A", "lower:b")]
                .iter()
                .any(|t| ev.meta.tags.is_subset(t))
        })
        .map(EventOrHeartbeat::Event)
        .collect::<Vec<_>>();
    expected.push(hb);

    assert_eq!(iter.next(), Some(expected));

    store.send(Input::Heartbeat(mk_test_heartbeat(stream_id, Offset::mk_test(0), None)));
    assert_eq!(iter.next(), Some(vec![]));

    let lamport = |x: u64| LamportTimestamp::from((store.top_offset() - Offset::ZERO) as u64 + x);
    let ev1 = mk_test_event(stream_id, store.top_offset() + 1, lamport(1));
    let ev2 = mk_test_event(stream_id, store.top_offset() + 2, lamport(2));
    let mut ev3 = mk_test_event(stream_id, store.top_offset() + 3, lamport(3));
    ev3.meta.tags = tags!("upper:B");

    // there should not be delivery with gaps, and no heartbeats derived from events
    store.send(Input::Events(vec![ev1.clone(), ev3.clone()]));
    assert_eq!(iter.next(), Some(vec![EventOrHeartbeat::Event(ev1)]));

    // there must be heartbeats derived from filtered events
    store.send(Input::Events(vec![ev2.clone()]));
    assert_eq!(
        iter.next(),
        Some(vec![
            EventOrHeartbeat::Event(ev2),
            EventOrHeartbeat::Heartbeat(StreamHeartBeat::from_event(&ev3))
        ])
    );

    assert_eq!(iter.next(), Some(vec![]));
}

#[tokio::test]
async fn test_harness_should_deliver_events_forward_min_large() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        OffsetOrMin::MIN,
        (store.top_offset() + 2).into(),
        vec![TagSet::empty()],
    );
    let mut iter = drainer(store.stream_forward(events, true)).await;

    assert_eq!(iter.next(), Some(store.known_events(stream_id, 0, None)));

    store.send(Input::Heartbeat(mk_test_heartbeat(stream_id, Offset::mk_test(0), None)));
    assert_eq!(iter.next(), Some(vec![]));

    let lamport = |x: u64| LamportTimestamp::from((store.top_offset() - Offset::ZERO) as u64 + x);

    let ev2 = mk_test_event(stream_id, store.top_offset() + 2, lamport(2));
    store.send(Input::Events(vec![ev2.clone()]));
    assert_eq!(iter.next(), Some(vec![]));

    let ev1 = mk_test_event(stream_id, store.top_offset() + 1, lamport(1));
    store.send(Input::Events(vec![ev1.clone()]));
    assert_eq!(iter.next(), Some(vec![ev1, ev2]));

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_forward_min_edge() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        OffsetOrMin::MIN,
        store.top_offset().into(),
        vec![TagSet::empty()],
    );
    let mut iter = drainer(store.stream_forward(events, true)).await;

    assert_eq!(iter.next(), Some(store.known_events(stream_id, 0, None)));

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_forward_min_small() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        OffsetOrMin::MIN,
        store.top_offset().pred_or_min(),
        vec![TagSet::empty()],
    );
    let mut iter = drainer(store.stream_forward(events, true)).await;

    assert_eq!(
        iter.next(),
        Some(store.known_events(stream_id, 0, Some((store.top_offset() - Offset::ZERO) as usize)))
    );

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_forward_min_zero() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        OffsetOrMin::MIN,
        OffsetOrMin::mk_test(0),
        vec![TagSet::empty()],
    );
    let mut iter = drainer(store.stream_forward(events, true)).await;

    assert_eq!(iter.next(), Some(store.known_events(stream_id, 0, Some(1))));

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_forward_min_min() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(stream_id, OffsetOrMin::MIN, OffsetOrMin::MIN, vec![TagSet::empty()]);
    let mut iter = drainer(store.stream_forward(events, true)).await;

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_forward_zero_min() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        OffsetOrMin::mk_test(0),
        OffsetOrMin::MIN,
        vec![TagSet::empty()],
    );
    let mut iter = drainer(store.stream_forward(events, true)).await;

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_forward_zero_small() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        OffsetOrMin::mk_test(0),
        OffsetOrMin::mk_test(10),
        vec![TagSet::empty()],
    );
    let mut iter = drainer(store.stream_forward(events, true)).await;

    assert_eq!(iter.next(), Some(store.known_events(stream_id, 1, Some(11))));

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_forward_zero_large() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        OffsetOrMin::mk_test(0),
        (store.top_offset() + 1).into(),
        vec![TagSet::empty()],
    );
    let mut iter = drainer(store.stream_forward(events, true)).await;

    let ev = mk_test_event(
        stream_id,
        store.top_offset() + 1,
        LamportTimestamp::from((store.top_offset() - Offset::ZERO) as u64 + 1),
    );
    store.send(Input::Events(vec![ev.clone()]));

    assert_eq!(
        iter.next(),
        Some(vec![store.known_events(stream_id, 1, None), vec![ev]].concat())
    );

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_forward_large_large() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        (store.top_offset() + 2).into(),
        (store.top_offset() + 4).into(),
        vec![TagSet::empty()],
    );
    let mut iter = drainer(store.stream_forward(events, true)).await;

    let evs = (1..6)
        .map(|x| {
            mk_test_event(
                stream_id,
                store.top_offset() + x,
                LamportTimestamp::from((store.top_offset() - Offset::ZERO) as u64 + x as u64),
            )
        })
        .collect::<Vec<_>>();
    store.send(Input::Events(vec![evs[0].clone()]));
    assert_eq!(iter.next(), Some(vec![]));

    // send one before and one within the subscription
    store.send(Input::Events(vec![evs[1].clone(), evs[2].clone()]));
    assert_eq!(iter.next(), Some(vec![evs[2].clone()]));

    // send one within and one after the subscription
    store.send(Input::Events(vec![evs[3].clone(), evs[4].clone()]));
    assert_eq!(iter.next(), Some(vec![evs[3].clone()]));

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_forward_large_large_evil() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        (store.top_offset() + 2).into(),
        (store.top_offset() + 4).into(),
        vec![TagSet::empty()],
    );
    let mut iter = drainer(store.stream_forward(events, true)).await;

    let evs = (1..6)
        .map(|x| {
            mk_test_event(
                stream_id,
                store.top_offset() + x,
                LamportTimestamp::from((store.top_offset() - Offset::ZERO) as u64 + x as u64),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(iter.next(), Some(vec![]));

    // send all but first event, where first is outside subscription
    store.send(Input::Events(evs.iter().cloned().skip(1).collect::<Vec<_>>()));
    assert_eq!(iter.next(), Some(vec![]));

    // finally send first event
    store.send(Input::Events(vec![evs[0].clone()]));
    assert_eq!(iter.next(), Some(vec![evs[2].clone(), evs[3].clone()]));

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_backward_min_edge() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        OffsetOrMin::MIN,
        store.top_offset().into(),
        vec![TagSet::empty()],
    );
    let stream = store.stream_backward(events).await.unwrap();
    let mut iter = Drainer::new(stream);

    assert_eq!(iter.next(), Some(store.known_events_rev(stream_id, 0, None)));

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_backward_min_small() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        OffsetOrMin::MIN,
        store.top_offset().pred_or_min(),
        vec![TagSet::empty()],
    );
    let stream = store.stream_backward(events).await.unwrap();
    let mut iter = Drainer::new(stream);

    assert_eq!(
        iter.next(),
        Some(store.known_events_rev(stream_id, 0, Some((store.top_offset() - Offset::ZERO) as usize)))
    );

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_backward_min_zero() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        OffsetOrMin::MIN,
        OffsetOrMin::mk_test(0),
        vec![TagSet::empty()],
    );
    let stream = store.stream_backward(events).await.unwrap();
    let mut iter = Drainer::new(stream);

    assert_eq!(iter.next(), Some(store.known_events_rev(stream_id, 0, Some(1))));

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_backward_min_min() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(stream_id, OffsetOrMin::MIN, OffsetOrMin::MIN, vec![TagSet::empty()]);
    let stream = store.stream_backward(events).await.unwrap();
    let mut iter = Drainer::new(stream);

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_backward_zero_min() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        OffsetOrMin::mk_test(0),
        OffsetOrMin::MIN,
        vec![TagSet::empty()],
    );
    let stream = store.stream_backward(events).await.unwrap();
    let mut iter = Drainer::new(stream);

    assert_eq!(iter.next(), None);
}

#[tokio::test]
async fn test_harness_should_deliver_events_backward_zero_small() {
    let store = TestEventStore::new();
    let stream_id = known_streams()[0];

    let events = StreamEventSelection::new(
        stream_id,
        OffsetOrMin::mk_test(0),
        OffsetOrMin::mk_test(10),
        vec![TagSet::empty()],
    );
    let stream = store.stream_backward(events).await.unwrap();
    let mut iter = Drainer::new(stream);

    assert_eq!(iter.next(), Some(store.known_events_rev(stream_id, 1, Some(11))));

    assert_eq!(iter.next(), None);
}
