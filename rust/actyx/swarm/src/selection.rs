use actyxos_sdk::{language::TagExpr, OffsetOrMin, StreamId};
use trees::query::TagsQuery;
use util::offsetmap_or_max::OffsetMapOrMax;

/// A precise selection of events, possibly unbounded in size.
///
/// Event selections consist of two parts:
///
///  - physical selection by quoting offsets from the streams of known stream IDs
///  - logical selection restricting which tags to admit, possibly also describing only from local streams
///
/// These two parts are combined in an AND fashion, intersecting the two selected
/// sets. If you want to filter logically, leave the OffsetMaps open. If you want to
/// filter physically, use TagSubscriptions::all(). If you want only certain events
/// from a fixed set of streams (e.g. when building a fully ordered stream), combine
/// the two mechanisms.
#[derive(Debug, Clone)]
pub struct EventSelection {
    /// Filtering events by tags
    pub tag_expr: TagExpr,
    /// Lower bound, exclusive, for all streams
    pub from_offsets_excluding: OffsetMapOrMax,
    /// Upper bound, inclusive, for all streams
    pub to_offsets_including: OffsetMapOrMax,
}

impl EventSelection {
    #[cfg(test)]
    pub fn matches<T>(&self, local: bool, event: &actyxos_sdk::Event<T>) -> bool {
        use actyxos_sdk::TagSet;
        let query = TagsQuery::from_expr(&self.tag_expr, "")(local);
        query.is_all()
            || query
                .terms()
                .any(|t| t.into_iter().cloned().collect::<TagSet>().is_subset(&event.meta.tags))
                && self.from_offsets_excluding.offset(event.key.stream) < event.key.offset
                && self.to_offsets_including.offset(event.key.stream) >= event.key.offset
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEventSelection {
    pub stream_id: StreamId,
    pub from_exclusive: OffsetOrMin,
    pub to_inclusive: OffsetOrMin,
    pub tags_query: TagsQuery,
}
