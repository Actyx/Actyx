use actyxos_sdk::{Event, Offset, OffsetOrMin, Payload, StreamId, TagSet};
use futures::{
    future::ready,
    stream::{self, Stream, StreamExt},
};
use num_traits::Bounded;
use std::collections::BTreeSet;
use trees::{OffsetMapOrMax, StreamHeartBeat, TagSubscriptions};

/// A precise selection of events, possibly unbounded in size.
///
/// Event selections consist of two parts:
///
///  - physical selection by quoting offsets from the streams of known stream IDs
///  - logical selection restricting which tags to admit, possibly also describing only from local streams
///
/// These two parts are combined in an AND fashion, intersecting the two selected
/// sets. If you want to filter logically, leave the PsnMaps open. If you want to
/// filter physically, use TagSubscriptions::all(). If you want only certain events
/// from a fixed set of streams (e.g. when building a fully ordered stream), combine
/// the two mechanisms.
#[derive(Debug, Clone)]
pub struct EventSelection {
    /// Filtering events by tag subsciption
    pub tag_subscriptions: TagSubscriptions,
    /// Lower bound, exclusive, for all streams
    pub from_offsets_excluding: OffsetMapOrMax,
    /// Upper bound, inclusive, for all streams
    pub to_offsets_including: OffsetMapOrMax,
}

impl EventSelection {
    pub fn new(
        tag_subscriptions: TagSubscriptions,
        from_offsets_excluding: OffsetMapOrMax,
        to_offsets_including: OffsetMapOrMax,
    ) -> EventSelection {
        EventSelection {
            tag_subscriptions,
            from_offsets_excluding,
            to_offsets_including,
        }
    }

    /// Select events matching the given logical subscription, with an inclusive upper
    /// bound in terms of Offset for each stream. The upper bound should normally be the
    /// “present” PsnMap obtained from the EventStore.
    pub fn upto(tag_subscriptions: TagSubscriptions, to_including: OffsetMapOrMax) -> EventSelection {
        Self::new(tag_subscriptions, OffsetMapOrMax::min_value(), to_including)
    }

    /// Select events matching the given logical subscription, with an exclusive lower
    /// bound in terms of Offset for each stream. The lower bound should normally be the
    /// “present” PsnMap obtained from the EventStore.
    pub fn after(tag_subscriptions: TagSubscriptions, from_excluding: OffsetMapOrMax) -> EventSelection {
        Self::new(tag_subscriptions, from_excluding, OffsetMapOrMax::max_value())
    }

    #[cfg(test)]
    pub fn create(query: &str, ranges: &[(StreamId, OffsetOrMin, OffsetOrMin)]) -> anyhow::Result<EventSelection> {
        let query = &query.parse::<actyxos_sdk::language::Query>()?;
        let tag_subscriptions = query.into();
        let from_offsets_excluding = OffsetMapOrMax::from_entries(
            ranges
                .iter()
                .cloned()
                .map(|(stream_id, from, _to)| (stream_id, from))
                .collect::<Vec<_>>()
                .as_ref(),
        );
        let to_offsets_including = OffsetMapOrMax::from_entries(
            ranges
                .iter()
                .cloned()
                .map(|(stream_id, _from, to)| (stream_id, to))
                .collect::<Vec<_>>()
                .as_ref(),
        );

        Ok(EventSelection {
            tag_subscriptions,
            from_offsets_excluding,
            to_offsets_including,
        })
    }
    #[cfg(test)]
    fn only_local(mut self) -> Self {
        for s in &mut self.tag_subscriptions.iter_mut() {
            s.local = true;
        }
        self
    }

    #[cfg(test)]
    pub fn matches<T>(&self, event: &Event<T>) -> bool {
        self.tag_subscriptions
            .iter()
            .any(|t| t.tags.is_subset(&event.meta.tags))
            && self.from_offsets_excluding.offset(event.key.stream) < event.key.offset
            && self.to_offsets_including.offset(event.key.stream) >= event.key.offset
    }

    /// Get all explicitly mentioned streams from the offset maps and subscription
    /// set, filtering out those with empty delivery intervals.
    pub fn get_mentioned_streams(&self, local_stream_ids: &BTreeSet<StreamId>) -> impl Iterator<Item = StreamId> + '_ {
        let mut stream_ids = BTreeSet::new();

        let from = &self.from_offsets_excluding;
        let to = &self.to_offsets_including;

        for stream_id in from.streams() {
            stream_ids.insert(stream_id);
        }
        for stream_id in to.streams() {
            stream_ids.insert(stream_id);
        }

        if self.tag_subscriptions.iter().any(|x| x.local) {
            stream_ids.append(&mut local_stream_ids.clone());
        }

        stream_ids.into_iter().filter(move |s| from.offset(*s) < to.offset(*s))
    }

    /// Returns true if the `from` and `to` offset maps can only differ for a finite set of streams,
    /// i.e. if their defaults do not permit positive event intervals.
    pub fn is_bounded(&self) -> bool {
        self.from_offsets_excluding.get_default() >= self.to_offsets_including.get_default()
    }

    pub fn bounded_streams(
        &self,
        store: &impl crate::event_store::EventStore,
    ) -> impl Iterator<Item = StreamEventSelection> + '_ {
        debug_assert!({
            self.to_offsets_including.streams().all(|s| {
                let from = self.from_offsets_excluding.offset(s);
                let to = self.to_offsets_including.offset(s);
                from < to
            })
        });
        let only_local = self.tag_subscriptions.only_local();
        let store = store.clone();
        self.to_offsets_including.streams().filter_map(move |stream_id| {
            let to = self.to_offsets_including.offset(stream_id);
            let from = self.from_offsets_excluding.offset(stream_id);
            let local = store.is_local(stream_id);

            if from < to && (!only_local || local) {
                Some(StreamEventSelection {
                    stream_id,
                    from_exclusive: from,
                    to_inclusive: to,
                    tag_subscriptions: self.tag_subscriptions.as_tag_sets(local),
                })
            } else {
                None
            }
        })
    }

    pub fn unbounded_stream(
        &self,
        store: &impl crate::event_store::EventStore,
        stream_id: StreamId,
    ) -> Option<StreamEventSelection> {
        let only_local = self.tag_subscriptions.only_local();
        let from = self.from_offsets_excluding.offset(stream_id);
        let local = store.is_local(stream_id);

        if !only_local || local {
            Some(StreamEventSelection {
                stream_id,
                from_exclusive: from,
                to_inclusive: OffsetOrMin::MAX,
                tag_subscriptions: self.tag_subscriptions.as_tag_sets(local),
            })
        } else {
            None
        }
    }

    /// Get a finite set of streams if possible. This can be thwarted by wildcard
    /// subscriptions coupled with wild-stream offset maps (i.e. those that admit an unbounded
    /// set of streams by having different default values).
    pub fn get_bounded_nonempty_streams(&self, local_stream_ids: &BTreeSet<StreamId>) -> Option<BTreeSet<StreamId>> {
        match (self.tag_subscriptions.only_local(), self.is_bounded()) {
            (true, _) => Some(local_stream_ids.clone()),
            (false, true) => Some(self.get_mentioned_streams(local_stream_ids).collect()),
            _ => None,
        }
    }

    pub fn for_stream(&self, stream_id: StreamId, is_local: bool) -> StreamEventSelection {
        StreamEventSelection {
            stream_id,
            from_exclusive: self.from_offsets_excluding.offset(stream_id),
            to_inclusive: self.to_offsets_including.offset(stream_id),
            tag_subscriptions: self.tag_subscriptions.as_tag_sets(is_local),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEventSelection {
    pub stream_id: StreamId,
    pub from_exclusive: OffsetOrMin,
    pub to_inclusive: OffsetOrMin,
    pub tag_subscriptions: Vec<TagSet>,
}
impl StreamEventSelection {
    pub fn new(
        stream_id: StreamId,
        from_exclusive: OffsetOrMin,
        to_inclusive: OffsetOrMin,
        tag_subscriptions: Vec<TagSet>,
    ) -> StreamEventSelection {
        StreamEventSelection {
            stream_id,
            from_exclusive,
            to_inclusive,
            tag_subscriptions,
        }
    }
}

#[derive(Clone, Debug)]
pub enum EventOrStop {
    Event(Event<Payload>),
    Stop(StreamId),
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventOrHeartbeat {
    Event(Event<Payload>),
    Heartbeat(StreamHeartBeat),
}

impl EventOrHeartbeat {
    pub fn into_event(self) -> Option<EventOrStop> {
        match self {
            EventOrHeartbeat::Event(x) => Some(EventOrStop::Event(x)),
            _ => None,
        }
    }

    pub fn offset(&self) -> Offset {
        match self {
            EventOrHeartbeat::Event(ev) => ev.key.offset,
            EventOrHeartbeat::Heartbeat(hb) => hb.offset,
        }
    }
}

pub fn stop_when_streams_exhausted(
    mut streams_to_go: BTreeSet<StreamId>,
    events: impl Stream<Item = EventOrStop> + Send,
) -> impl Stream<Item = Event<Payload>> + Send {
    if streams_to_go.is_empty() {
        return stream::empty().left_stream();
    }
    events
        .take_while(move |eos| match eos {
            EventOrStop::Stop(stream_id) => {
                streams_to_go.remove(&stream_id);
                // the last Stop will terminate the stream
                ready(!streams_to_go.is_empty())
            }
            _ => ready(true),
        })
        .filter_map(|eos| match eos {
            EventOrStop::Event(ev) => ready(Some(ev)),
            _ => ready(None),
        })
        .right_stream()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::tests::*;
    use actyxos_sdk::{language, tags};
    use maplit::btreeset;
    use pretty_assertions::assert_eq;
    use trees::TagSubscription;

    fn stream_ids(range: &[usize]) -> BTreeSet<StreamId> {
        range.iter().map(|i| test_stream(*i)).collect()
    }

    #[test]
    fn event_selection_must_list_stream_ids_wildcard_bounded() {
        let selection = EventSelection::create(
            "FROM 'upper:A' & 'lower:a'",
            &[(test_stream(0), OffsetOrMin::MIN, OffsetOrMin::MAX)],
        )
        .unwrap();
        let empty = BTreeSet::new();
        let expected = stream_ids(&[0]);
        assert_eq!(selection.get_bounded_nonempty_streams(&empty), Some(expected.clone()));
        assert_eq!(
            selection.get_mentioned_streams(&empty).collect::<BTreeSet<_>>(),
            expected
        );
    }

    #[test]
    fn event_selection_must_list_stream_ids_wildcard_unbounded() {
        let query = &"FROM 'upper:A' & 'lower:a'".parse::<language::Query>().unwrap();
        let selection = EventSelection {
            tag_subscriptions: query.into(),
            from_offsets_excluding: OffsetMapOrMax::from_entries(&[
                (test_stream(0), OffsetOrMin::MIN),
                (test_stream(1), OffsetOrMin::mk_test(50)),
                (test_stream(2), OffsetOrMin::MAX),
            ]),
            to_offsets_including: OffsetMapOrMax::max_value(),
        };
        let empty = BTreeSet::new();
        assert_eq!(selection.get_bounded_nonempty_streams(&empty), None);
        assert_eq!(
            selection.get_mentioned_streams(&empty).collect::<BTreeSet<_>>(),
            stream_ids(&[1])
        );
    }

    #[test]
    fn event_selection_must_list_stream_ids_local_bounded() {
        let events = EventSelection::create(
            "FROM 'upper:A' & 'lower:a'",
            &[
                (test_stream(0), OffsetOrMin::MIN, OffsetOrMin::MAX),
                (test_stream(1), OffsetOrMin::MIN, OffsetOrMin::MAX),
            ],
        )
        .expect("cannot construct selection")
        .only_local();
        let local = btreeset! { test_stream(0) };
        assert_eq!(events.get_bounded_nonempty_streams(&local), Some(stream_ids(&[0])));
        assert_eq!(
            events.get_mentioned_streams(&local).collect::<BTreeSet<_>>(),
            stream_ids(&[0, 1])
        );
    }

    #[test]
    fn event_selection_must_list_stream_ids_local_unbounded() {
        let query = &"FROM 'upper:A' & 'lower:a'".parse::<language::Query>().unwrap();
        let events = EventSelection::new(
            query.into(),
            OffsetMapOrMax::from_entries(&[
                (test_stream(0), OffsetOrMin::MIN),
                (test_stream(1), OffsetOrMin::MIN),
                (test_stream(2), OffsetOrMin::MAX),
            ]),
            OffsetMapOrMax::max_value(),
        )
        .only_local();
        let local = stream_ids(&[0]);
        assert_eq!(events.get_bounded_nonempty_streams(&local), Some(local.clone()));
        assert_eq!(events.get_mentioned_streams(&local).collect::<BTreeSet<_>>(), local);
    }

    #[test]
    fn event_selection_must_filter_for_local_stream() {
        let tag_subscriptions = TagSubscriptions::new(vec![
            TagSubscription::new(tags!("upper:A", "lower:a")).local(),
            TagSubscription::new(tags!("upper:B")),
        ]);

        let events = EventSelection::new(
            tag_subscriptions,
            OffsetMapOrMax::from_entries(&[
                (test_stream(0), OffsetOrMin::MIN),
                (test_stream(1), OffsetOrMin::MIN),
                (test_stream(2), OffsetOrMin::MAX),
            ]),
            OffsetMapOrMax::from_entries(&[
                (test_stream(0), OffsetOrMin::mk_test(50)),
                (test_stream(1), OffsetOrMin::MIN),
                (test_stream(2), OffsetOrMin::MAX),
            ]),
        );
        let set = vec![tags!("upper:B")];
        let expected1 = StreamEventSelection::new(test_stream(1), OffsetOrMin::MIN, OffsetOrMin::MIN, set.clone());
        let expected2 = StreamEventSelection::new(test_stream(2), OffsetOrMin::MAX, OffsetOrMin::MAX, set);
        assert_eq!(events.for_stream(test_stream(1), false), expected1);
        assert_eq!(events.for_stream(test_stream(2), false), expected2);
    }
}
