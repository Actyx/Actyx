use actyx_sdk::{language::TagExpr, OffsetMap, OffsetOrMin, StreamId};
use trees::query::TagExprQuery;

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
    pub from_offsets_excluding: OffsetMap,
    /// Upper bound, inclusive, for all streams
    pub to_offsets_including: OffsetMap,
}

impl EventSelection {
    #[cfg(test)]
    pub fn matches<T>(&self, local: bool, event: &actyx_sdk::Event<T>) -> bool {
        use actyx_sdk::TagSet;
        let query = TagExprQuery::from_expr(&self.tag_expr).unwrap()(local, event.key.stream);
        query.is_all()
            || query.terms().any(|t| {
                t.into_iter()
                    .filter_map(|t| t.to_app())
                    .cloned()
                    .collect::<TagSet>()
                    .is_subset(&event.meta.tags)
            }) && self.from_offsets_excluding.offset(event.key.stream) < event.key.offset
                && self.to_offsets_including.offset(event.key.stream) >= event.key.offset
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEventSelection {
    pub stream_id: StreamId,
    pub from_exclusive: OffsetOrMin,
    pub to_inclusive: OffsetOrMin,
    pub tags_query: TagExprQuery,
}
