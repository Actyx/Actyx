use actyxos_sdk::{OffsetOrMin, StreamId, TagSet};
use num_traits::Bounded;
use trees::{OffsetMapOrMax, TagSubscriptions};

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
    pub fn matches<T>(&self, event: &actyxos_sdk::Event<T>) -> bool {
        self.tag_subscriptions
            .iter()
            .any(|t| t.tags.is_subset(&event.meta.tags))
            && self.from_offsets_excluding.offset(event.key.stream) < event.key.offset
            && self.to_offsets_including.offset(event.key.stream) >= event.key.offset
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
