use actyxos_sdk::{
    tagged::{Event, StreamId, TagSet},
    Expression, Offset, OffsetOrMin, Payload,
};
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
/// from a fixed set of sources (e.g. when building a fully ordered stream), combine
/// the two mechanisms.
#[derive(Debug, Clone)]
pub struct EventSelection {
    /// Filtering events by semantics, name, sourceId, and possibly payload
    pub subscription_set: TagSubscriptions,
    /// Lower bound, exclusive, for all sources
    pub from_offsets_excluding: OffsetMapOrMax,
    /// Upper bound, inclusive, for all sources
    pub to_offsets_including: OffsetMapOrMax,
}

impl EventSelection {
    pub fn new(
        subscription_set: TagSubscriptions,
        from_offsets_excluding: OffsetMapOrMax,
        to_offsets_including: OffsetMapOrMax,
    ) -> EventSelection {
        EventSelection {
            subscription_set,
            from_offsets_excluding,
            to_offsets_including,
        }
    }

    /// Select events matching the given logical subscription, with an inclusive upper
    /// bound in terms of Offset for each source. The upper bound should normally be the
    /// “present” PsnMap obtained from the EventStore.
    pub fn upto(subscription_set: TagSubscriptions, to_including: OffsetMapOrMax) -> EventSelection {
        Self::new(subscription_set, OffsetMapOrMax::min_value(), to_including)
    }

    /// Select events matching the given logical subscription, with an exclusive lower
    /// bound in terms of Offset for each source. The lower bound should normally be the
    /// “present” PsnMap obtained from the EventStore.
    pub fn after(subscription_set: TagSubscriptions, from_excluding: OffsetMapOrMax) -> EventSelection {
        Self::new(subscription_set, from_excluding, OffsetMapOrMax::max_value())
    }

    pub fn tag_expr(
        query: String,
        from_offsets_excluding: OffsetMapOrMax,
        to_offsets_including: OffsetMapOrMax,
    ) -> anyhow::Result<EventSelection> {
        let subscription_set = query.parse::<Expression>()?.into();
        Ok(EventSelection {
            subscription_set,
            from_offsets_excluding,
            to_offsets_including,
        })
    }

    #[cfg(test)]
    pub fn create(tag_expr: &str, ranges: &[(StreamId, OffsetOrMin, OffsetOrMin)]) -> anyhow::Result<EventSelection> {
        let subscription_set = tag_expr.parse::<actyxos_sdk::Expression>()?.into();
        let from_offsets_excluding = OffsetMapOrMax::from_entries(
            ranges
                .iter()
                .cloned()
                .map(|(source, from, _to)| (source, from))
                .collect::<Vec<_>>()
                .as_ref(),
        );
        let to_offsets_including = OffsetMapOrMax::from_entries(
            ranges
                .iter()
                .cloned()
                .map(|(source, _from, to)| (source, to))
                .collect::<Vec<_>>()
                .as_ref(),
        );

        Ok(EventSelection {
            subscription_set,
            from_offsets_excluding,
            to_offsets_including,
        })
    }
    #[cfg(test)]
    fn only_local(mut self) -> Self {
        for s in &mut self.subscription_set.iter_mut() {
            s.local = true;
        }
        self
    }

    #[cfg(test)]
    pub fn matches<T>(&self, event: &Event<T>) -> bool {
        self.subscription_set.iter().any(|t| t.tags.is_subset(&event.meta.tags))
            && self.from_offsets_excluding.offset(event.key.stream) < event.key.offset
            && self.to_offsets_including.offset(event.key.stream) >= event.key.offset
    }

    /// Get all explicitly mentioned sources from the PsnMaps and subscription
    /// set, filtering out those with empty delivery intervals.
    pub fn get_mentioned_streams(&self, local_stream_ids: &BTreeSet<StreamId>) -> impl Iterator<Item = StreamId> + '_ {
        let mut sources = BTreeSet::new();

        let from = &self.from_offsets_excluding;
        let to = &self.to_offsets_including;

        for source in from.streams() {
            sources.insert(source);
        }
        for source in to.streams() {
            sources.insert(source);
        }

        if self.subscription_set.iter().any(|x| x.local) {
            sources.append(&mut local_stream_ids.clone());
        }

        sources.into_iter().filter(move |s| from.offset(*s) < to.offset(*s))
    }

    /// Returns true if the `from` and `to` PsnMaps can only differ for a finite set of sources,
    /// i.e. if their defaults do not permit positive event intervals.
    pub fn is_bounded(&self) -> bool {
        self.from_offsets_excluding.get_default() >= self.to_offsets_including.get_default()
    }

    /// Get a finite set of sources if possible. This can be thwarted by wildcard
    /// subscriptions coupled with wild-source PsnMaps (i.e. those that admit an unbounded
    /// set of sources by having different default values).
    pub fn get_bounded_nonempty_streams(&self, local_stream_ids: &BTreeSet<StreamId>) -> Option<BTreeSet<StreamId>> {
        match (self.subscription_set.only_local(), self.is_bounded()) {
            (true, _) => Some(local_stream_ids.clone()),
            (false, true) => Some(self.get_mentioned_streams(local_stream_ids).collect()),
            _ => None,
        }
    }

    pub fn for_stream(&self, stream_id: StreamId, is_local: bool) -> StreamEventSelection {
        let subscription_set = self
            .subscription_set
            .iter()
            // if `stream_id` is a local stream, we can just use all
            // subscriptions. Otherwise, only non-local subscriptions.
            .filter(|x| is_local || !x.local)
            .cloned()
            .map(|x| x.tags)
            .collect();
        StreamEventSelection {
            stream_id,
            from_exclusive: self.from_offsets_excluding.offset(stream_id),
            to_inclusive: self.to_offsets_including.offset(stream_id),
            subscription_set,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEventSelection {
    pub stream_id: StreamId,
    pub from_exclusive: OffsetOrMin,
    pub to_inclusive: OffsetOrMin,
    pub subscription_set: Vec<TagSet>,
}
impl StreamEventSelection {
    pub fn new(
        stream_id: StreamId,
        from_exclusive: OffsetOrMin,
        to_inclusive: OffsetOrMin,
        subscription_set: Vec<TagSet>,
    ) -> StreamEventSelection {
        StreamEventSelection {
            stream_id,
            from_exclusive,
            to_inclusive,
            subscription_set,
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
                // the last Stop(source) will terminate the stream
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
    use actyxos_sdk::{tags, Expression};
    use maplit::btreeset;
    use pretty_assertions::assert_eq;
    use trees::TagSubscription;

    fn sources(range: &[usize]) -> BTreeSet<StreamId> {
        range.iter().map(|i| source(*i)).collect()
    }

    #[test]
    fn event_selection_must_list_sources_wildcard_bounded() {
        let events = EventSelection::create(
            "'upper:A' & 'lower:a'",
            &[(source(0), OffsetOrMin::MIN, OffsetOrMin::MAX)],
        )
        .expect("cannot construct selection");
        let empty = BTreeSet::new();
        let expected = sources(&[0]);
        assert_eq!(events.get_bounded_nonempty_streams(&empty), Some(expected.clone()));
        assert_eq!(events.get_mentioned_streams(&empty).collect::<BTreeSet<_>>(), expected);
    }

    #[test]
    fn event_selection_must_list_sources_wildcard_unbounded() {
        let events = EventSelection::tag_expr(
            "'upper:A' & 'lower:a'".into(),
            OffsetMapOrMax::from_entries(&[
                (source(0), OffsetOrMin::MIN),
                (source(1), OffsetOrMin::mk_test(50)),
                (source(2), OffsetOrMin::MAX),
            ]),
            OffsetMapOrMax::max_value(),
        )
        .unwrap();
        let expected = sources(&[1]);
        let empty = BTreeSet::new();
        assert_eq!(events.get_bounded_nonempty_streams(&empty), None);
        assert_eq!(events.get_mentioned_streams(&empty).collect::<BTreeSet<_>>(), expected);
    }

    #[test]
    fn event_selection_must_list_sources_local_bounded() {
        let events = EventSelection::create(
            "'upper:A' & 'lower:a'",
            &[
                (source(0), OffsetOrMin::MIN, OffsetOrMin::MAX),
                (source(1), OffsetOrMin::MIN, OffsetOrMin::MAX),
            ],
        )
        .expect("cannot construct selection")
        .only_local();
        let local = btreeset! { source(0) };
        assert_eq!(events.get_bounded_nonempty_streams(&local), Some(sources(&[0])));
        assert_eq!(
            events.get_mentioned_streams(&local).collect::<BTreeSet<_>>(),
            sources(&[0, 1])
        );
    }

    #[test]
    fn event_selection_must_list_sources_local_unbounded() {
        let events = EventSelection::new(
            "'upper:A' & 'lower:a'".parse::<Expression>().unwrap().into(),
            OffsetMapOrMax::from_entries(&[
                (source(0), OffsetOrMin::MIN),
                (source(1), OffsetOrMin::MIN),
                (source(2), OffsetOrMin::MAX),
            ]),
            OffsetMapOrMax::max_value(),
        )
        .only_local();
        let local = sources(&[0]);
        assert_eq!(events.get_bounded_nonempty_streams(&local), Some(local.clone()));
        assert_eq!(events.get_mentioned_streams(&local).collect::<BTreeSet<_>>(), local);
    }

    #[test]
    fn event_selection_must_filter_for_local_stream() {
        let subscription_set = TagSubscriptions::new(vec![
            TagSubscription::new(tags!("'upper:A' & 'lower:a'")).local(),
            TagSubscription::new(tags!("upper:B")),
        ]);

        let events = EventSelection::new(
            subscription_set,
            OffsetMapOrMax::from_entries(&[
                (source(0), OffsetOrMin::MIN),
                (source(1), OffsetOrMin::MIN),
                (source(2), OffsetOrMin::MAX),
            ]),
            OffsetMapOrMax::from_entries(&[
                (source(0), OffsetOrMin::mk_test(50)),
                (source(1), OffsetOrMin::MIN),
                (source(2), OffsetOrMin::MAX),
            ]),
        );
        let set = vec![tags!("upper:B")];
        let expected1 = StreamEventSelection::new(source(1), OffsetOrMin::MIN, OffsetOrMin::MIN, set.clone());
        let expected2 = StreamEventSelection::new(source(2), OffsetOrMin::MAX, OffsetOrMin::MAX, set);
        assert_eq!(events.for_stream(source(1), false), expected1);
        assert_eq!(events.for_stream(source(2), false), expected2);
    }
}
