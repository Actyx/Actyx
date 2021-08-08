use std::{
    cmp::Ord,
    collections::BTreeSet,
    iter::{once, FromIterator},
    ops::{BitAndAssign, Range, RangeFrom, RangeTo},
};

use actyx_sdk::{
    language::{self, SortKey, TagAtom},
    tag, StreamId, Timestamp,
};
use banyan::{
    index::{BranchIndex, CompactSeq, LeafIndex},
    query::Query,
};
use cbor_tag_index::DnfQuery;
use range_collections::RangeSet;

use crate::{
    axtrees::{AxTrees, TagsSummaries},
    dnf::Dnf,
    tags::{ScopedTag, ScopedTagSet, TagScope},
};

#[derive(Debug, derive_more::Display, derive_more::Error, Clone)]
pub enum TagExprError {
    #[display(fmt = "Lamport timestamp restrictions must be the same on all branches")]
    InconsistentLamport,
    #[display(fmt = "Timestamp restrictions must be the same on all branches")]
    InconsistentTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LamportQueryBuilder(RangeSet<SortKey>);

impl LamportQueryBuilder {
    pub fn all() -> Self {
        Self(RangeSet::all())
    }
    pub fn empty() -> Self {
        Self(RangeSet::empty())
    }
    pub fn is_all(&self) -> bool {
        self.0.is_all()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn build(&self, stream: StreamId) -> LamportQuery {
        LamportQuery(self.0.clone(), stream)
    }
}

impl BitAndAssign for LamportQueryBuilder {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0.bitand_assign(rhs.0)
    }
}

impl From<Range<SortKey>> for LamportQueryBuilder {
    fn from(value: Range<SortKey>) -> Self {
        Self(value.into())
    }
}

impl From<RangeSet<SortKey>> for LamportQueryBuilder {
    fn from(value: RangeSet<SortKey>) -> Self {
        Self(value)
    }
}

impl From<RangeFrom<SortKey>> for LamportQueryBuilder {
    fn from(value: RangeFrom<SortKey>) -> Self {
        Self(value.into())
    }
}

impl From<RangeTo<SortKey>> for LamportQueryBuilder {
    fn from(value: RangeTo<SortKey>) -> Self {
        Self(value.into())
    }
}

impl FromIterator<LamportQueryBuilder> for LamportQueryBuilder {
    fn from_iter<T: IntoIterator<Item = LamportQueryBuilder>>(iter: T) -> Self {
        let mut ret = Self::all();
        for q in iter.into_iter() {
            ret &= q;
        }
        ret
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LamportQuery(RangeSet<SortKey>, StreamId);

impl LamportQuery {
    pub fn all() -> Self {
        Self(RangeSet::all(), StreamId::min())
    }
    pub fn empty() -> Self {
        Self(RangeSet::empty(), StreamId::min())
    }
    pub fn is_all(&self) -> bool {
        self.0.is_all()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Query<AxTrees> for LamportQuery {
    fn intersecting(&self, _offset: u64, index: &BranchIndex<AxTrees>, matching: &mut [bool]) {
        if self.0.is_all() {
            return;
        }
        let lamport = &index.summaries.lamport;
        for i in 0..lamport.len().min(matching.len()) {
            if matching[i] {
                let min = lamport[i].min;
                // this is somewhat too large: the best upper bound would be to increment the SortKey
                // but that is quite a bit more complexity for little gain; the real event filter below
                // is perfectly precise, so the effect is that we may inspect more leaves than needed
                let max = lamport[i].max + 1;
                let r = RangeSet::from(SortKey::new(min, self.1)..SortKey::new(max, self.1));
                matching[i] = !self.0.is_disjoint(&r);
            }
        }
    }

    fn containing(&self, _offset: u64, index: &LeafIndex<AxTrees>, matching: &mut [bool]) {
        if self.0.is_all() {
            return;
        }
        let lamport = &index.keys.lamport;
        for i in 0..lamport.len().min(matching.len()) {
            if matching[i] {
                let key = SortKey::new(lamport[i], self.1);
                matching[i] = self.0.contains(&key);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeQuery(RangeSet<Timestamp>);

impl TimeQuery {
    pub fn all() -> Self {
        Self(RangeSet::all())
    }
    pub fn empty() -> Self {
        Self(RangeSet::empty())
    }
    pub fn is_all(&self) -> bool {
        self.0.is_all()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl BitAndAssign for TimeQuery {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0.bitand_assign(rhs.0)
    }
}

impl From<Range<Timestamp>> for TimeQuery {
    fn from(value: Range<Timestamp>) -> Self {
        Self(value.into())
    }
}

impl From<RangeSet<Timestamp>> for TimeQuery {
    fn from(value: RangeSet<Timestamp>) -> Self {
        Self(value)
    }
}

impl From<RangeFrom<Timestamp>> for TimeQuery {
    fn from(value: RangeFrom<Timestamp>) -> Self {
        Self(value.into())
    }
}

impl From<RangeTo<Timestamp>> for TimeQuery {
    fn from(value: RangeTo<Timestamp>) -> Self {
        Self(value.into())
    }
}

impl FromIterator<TimeQuery> for TimeQuery {
    fn from_iter<T: IntoIterator<Item = TimeQuery>>(iter: T) -> Self {
        let mut ret = Self::all();
        for q in iter.into_iter() {
            ret &= q;
        }
        ret
    }
}

impl Query<AxTrees> for TimeQuery {
    fn intersecting(&self, _offset: u64, index: &BranchIndex<AxTrees>, matching: &mut [bool]) {
        if self.0.is_all() {
            return;
        }
        let time = &index.summaries.time;
        for i in 0..time.len().min(matching.len()) {
            matching[i] = matching[i] && !self.0.is_disjoint(&time[i].into());
        }
    }

    fn containing(&self, _offset: u64, index: &LeafIndex<AxTrees>, matching: &mut [bool]) {
        if self.0.is_all() {
            return;
        }
        let time = &index.keys.time;
        for i in 0..time.len().min(matching.len()) {
            matching[i] = matching[i] && self.0.contains(&time[i]);
        }
    }
}

#[derive(Debug, Clone)]
pub struct OffsetQuery(RangeSet<u64>);

impl From<Range<u64>> for OffsetQuery {
    fn from(value: Range<u64>) -> Self {
        Self(value.into())
    }
}
impl From<RangeSet<u64>> for OffsetQuery {
    fn from(value: RangeSet<u64>) -> Self {
        Self(value)
    }
}

impl From<RangeFrom<u64>> for OffsetQuery {
    fn from(value: RangeFrom<u64>) -> Self {
        Self(value.into())
    }
}

impl From<RangeTo<u64>> for OffsetQuery {
    fn from(value: RangeTo<u64>) -> Self {
        Self(value.into())
    }
}

impl Query<AxTrees> for OffsetQuery {
    fn intersecting(&self, offset: u64, index: &BranchIndex<AxTrees>, matching: &mut [bool]) {
        let range = offset..offset + index.count;
        if self.0.is_disjoint(&range.into()) {
            for e in matching.iter_mut() {
                *e = false;
            }
        }
    }

    fn containing(&self, mut offset: u64, index: &LeafIndex<AxTrees>, matching: &mut [bool]) {
        for i in 0..index.keys.len().min(matching.len()) {
            matching[i] = matching[i] && self.0.contains(&offset);
            offset += 1;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagExprQuery {
    tags: DnfQuery<ScopedTag>,
    lamport: LamportQuery,
    time: TimeQuery,
}

impl TagExprQuery {
    pub fn new(terms: impl IntoIterator<Item = ScopedTagSet>, lamport: LamportQuery, time: TimeQuery) -> Self {
        let tags = DnfQuery::new(terms).expect("> u32::max_value() tags");
        let lamport = if tags.is_empty() {
            LamportQuery::empty()
        } else {
            lamport
        };
        let time = if tags.is_empty() { TimeQuery::empty() } else { time };
        Self { tags, lamport, time }
    }

    pub fn from_expr(tag_expr: &language::TagExpr) -> Result<impl Fn(bool, StreamId) -> Self, TagExprError> {
        let dnf = Dnf::from(tag_expr).0;

        let mut terms = vec![];
        let mut local_terms = vec![];
        let no_terms = vec![];

        let mut lamport = None;
        let mut time = None;

        for tag_set in &dnf {
            let tags = {
                match get_scoped_tags(tag_set) {
                    Some(value) => value,
                    None => continue,
                }
            };
            let is_local = tag_set.iter().any(|x| x.is_local());
            let target = if is_local { &mut local_terms } else { &mut terms };
            if tags.is_empty() {
                // clear other OR branches because this branch tag-matches all events
                *target = vec![tags];
            } else if target.get(0) == Some(&ScopedTagSet::empty()) {
                // no point adding an OR branch, we’ll match all events anyway
            } else {
                target.push(tags);
            }
            get_lamport_query(tag_set, &mut lamport)?;
            get_time_query(tag_set, &mut time)?;
        }

        let lamport = lamport.unwrap_or_else(LamportQueryBuilder::all);
        let time = time.unwrap_or_else(TimeQuery::all);

        Ok(move |local, stream| {
            let mut local = (if local { local_terms.iter() } else { no_terms.iter() })
                .cloned()
                .peekable();
            if terms.get(0) == Some(&ScopedTagSet::empty()) || local.peek() == Some(&ScopedTagSet::empty()) {
                Self::new(once(ScopedTagSet::empty()), lamport.build(stream), time.clone())
            } else {
                Self::new(terms.iter().cloned().chain(local), lamport.build(stream), time.clone())
            }
        })
    }

    pub fn all() -> Self {
        Self {
            tags: DnfQuery::all(),
            lamport: LamportQuery::all(),
            time: TimeQuery::all(),
        }
    }

    pub fn empty() -> Self {
        Self {
            tags: DnfQuery::empty(),
            lamport: LamportQuery::empty(),
            time: TimeQuery::empty(),
        }
    }

    pub fn terms(&self) -> impl Iterator<Item = impl IntoIterator<Item = &ScopedTag>> {
        self.tags.terms()
    }

    pub fn is_all(&self) -> bool {
        self.tags.is_all() && self.lamport.is_all() && self.time.is_all()
    }

    pub fn is_empty(&self) -> bool {
        self.tags.is_empty() || self.lamport.is_empty() || self.time.is_empty()
    }
}

fn get_lamport_query(tag_set: &BTreeSet<TagAtom>, q: &mut Option<LamportQueryBuilder>) -> Result<(), TagExprError> {
    let query = tag_set
        .iter()
        .filter_map(|x| match x {
            TagAtom::FromLamport(l) => Some(LamportQueryBuilder::from(*l..)),
            TagAtom::ToLamport(l) => Some(LamportQueryBuilder::from(..*l)),
            _ => None,
        })
        .collect();
    match q {
        Some(q) => {
            if *q == query {
                Ok(())
            } else {
                Err(TagExprError::InconsistentLamport)
            }
        }
        None => {
            *q = Some(query);
            Ok(())
        }
    }
}

fn get_time_query(tag_set: &BTreeSet<TagAtom>, q: &mut Option<TimeQuery>) -> Result<(), TagExprError> {
    let query = tag_set
        .iter()
        .filter_map(|x| match x {
            TagAtom::FromTime(l) => Some(TimeQuery::from(*l..)),
            TagAtom::ToTime(l) => Some(TimeQuery::from(..*l)),
            _ => None,
        })
        .collect();
    match q {
        Some(q) => {
            if *q == query {
                Ok(())
            } else {
                Err(TagExprError::InconsistentTime)
            }
        }
        None => {
            *q = Some(query);
            Ok(())
        }
    }
}

fn get_scoped_tags(tag_set: &BTreeSet<TagAtom>) -> Option<ScopedTagSet> {
    let app_id: ScopedTagSet = get_app_id(tag_set);
    if app_id.len() > 1 {
        // an event can never have two different app IDs
        return None;
    }
    let mut tags: ScopedTagSet = get_tags(tag_set);
    if let Some(app_id) = app_id.into_iter().next() {
        tags.insert(app_id);
    }
    Some(tags)
}

fn get_tags(tag_set: &BTreeSet<TagAtom>) -> ScopedTagSet {
    tag_set
        .iter()
        .filter_map(|x| x.tag())
        .map(|tag| ScopedTag::from(tag.clone()))
        .collect()
}

fn get_app_id(tag_set: &BTreeSet<TagAtom>) -> ScopedTagSet {
    tag_set
        .iter()
        .filter_map(|x| {
            if let TagAtom::AppId(id) = x {
                Some(ScopedTag::new(TagScope::Internal, tag!("app_id:") + id.as_str()))
            } else {
                None
            }
        })
        .collect()
}

impl Query<AxTrees> for TagExprQuery {
    fn containing(&self, offset: u64, index: &LeafIndex<AxTrees>, matching: &mut [bool]) {
        self.lamport.containing(offset, index, matching);
        self.time.containing(offset, index, matching);
        self.tags.set_matching(&index.keys.tags, matching);
    }

    fn intersecting(&self, offset: u64, index: &BranchIndex<AxTrees>, matching: &mut [bool]) {
        self.lamport.intersecting(offset, index, matching);
        self.time.intersecting(offset, index, matching);
        if let TagsSummaries::Complete(index) = &index.summaries.tags {
            self.tags.set_matching(index, matching);
        }
    }
}

impl FromIterator<ScopedTagSet> for TagExprQuery {
    fn from_iter<T: IntoIterator<Item = ScopedTagSet>>(iter: T) -> Self {
        Self::new(iter, LamportQuery::all(), TimeQuery::all())
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use crate::{stags, TagIndex};

    use super::*;
    use actyx_sdk::{
        language::{TagAtom, TagExpr},
        tags, NodeId, Tag,
    };

    fn l(tag: &'static str) -> TagExpr {
        TagExpr::Atom(TagAtom::Tag(Tag::from_str(tag).unwrap()))
    }

    fn assert_match(index: &TagIndex, expr: &TagExpr, expected: Vec<bool>) {
        let query = TagExprQuery::from_expr(expr).unwrap()(true, StreamId::min());
        let mut matching = vec![true; expected.len()];
        query.tags.set_matching(index, &mut matching);
        assert_eq!(matching, expected);
    }

    #[test]
    fn test_matching_1() {
        let index = TagIndex::new(vec![stags!("a"), stags!("a", "b"), stags!("a"), stags!("a", "b")]).unwrap();
        let expr = l("a") | l("b");
        assert_match(&index, &expr, vec![true, true, true, true]);
        let expr = l("a") & l("b");
        assert_match(&index, &expr, vec![false, true, false, true]);
        let expr = l("c") & l("d");
        assert_match(&index, &expr, vec![false, false, false, false]);
    }

    #[test]
    fn test_matching_2() {
        let index = TagIndex::new(vec![
            stags!("a", "b"),
            stags!("b", "c"),
            stags!("c", "a"),
            stags!("a", "b"),
        ])
        .unwrap();
        let expr = l("a") | l("b") | l("c");
        assert_match(&index, &expr, vec![true, true, true, true]);
        let expr = l("a") & l("b");
        assert_match(&index, &expr, vec![true, false, false, true]);
        let expr = l("a") & l("b") & l("c");
        assert_match(&index, &expr, vec![false, false, false, false]);
    }

    #[test]
    fn test_from_expr() {
        let test_expr = |local: bool, tag_expr: &'static str, expected: TagExprQuery| {
            let tag_expr = tag_expr.parse::<TagExpr>().unwrap();
            let actual = TagExprQuery::from_expr(&tag_expr).unwrap()(local, StreamId::min());
            assert_eq!(actual, expected, "tag_expr: {:?}", tag_expr);
        };

        test_expr(true, "allEvents", TagExprQuery::all());
        test_expr(true, "allEvents | 'a'", TagExprQuery::all());
        test_expr(true, "allEvents | isLocal", TagExprQuery::all());
        test_expr(true, "allEvents & isLocal", TagExprQuery::all());
        test_expr(true, "isLocal", TagExprQuery::all());
        test_expr(
            true,
            "isLocal & 'a'",
            TagExprQuery::new(vec![stags!("a")], LamportQuery::all(), TimeQuery::all()),
        );
        test_expr(true, "isLocal | 'a'", TagExprQuery::all());
        test_expr(
            true,
            "isLocal & 'b' | 'a'",
            TagExprQuery::new(vec![stags!("a"), stags!("b")], LamportQuery::all(), TimeQuery::all()),
        );
        test_expr(
            true,
            "'a'",
            TagExprQuery::new(vec![stags!("a")], LamportQuery::all(), TimeQuery::all()),
        );

        test_expr(false, "allEvents", TagExprQuery::all());
        test_expr(false, "allEvents | 'a'", TagExprQuery::all());
        test_expr(false, "allEvents | isLocal", TagExprQuery::all());
        test_expr(false, "allEvents & isLocal", TagExprQuery::empty());
        test_expr(false, "isLocal", TagExprQuery::empty());
        test_expr(false, "isLocal & 'a'", TagExprQuery::empty());
        test_expr(
            false,
            "isLocal | 'a'",
            TagExprQuery::new(vec![tags!("a").into()], LamportQuery::all(), TimeQuery::all()),
        );
        test_expr(
            false,
            "isLocal & 'b' | 'a'",
            TagExprQuery::new(vec![tags!("a").into()], LamportQuery::all(), TimeQuery::all()),
        );
        test_expr(
            false,
            "'a'",
            TagExprQuery::new(vec![tags!("a").into()], LamportQuery::all(), TimeQuery::all()),
        );
    }

    fn dnf(s: &str) -> Dnf {
        Dnf::from(&s.parse::<TagExpr>().unwrap())
    }
    fn tag_set(s: &str) -> BTreeSet<TagAtom> {
        let mut it = dnf(s).0.into_iter();
        let ret = it.next().unwrap();
        assert!(it.next().is_none());
        ret
    }

    #[test]
    fn app_id() {
        assert_eq!(get_app_id(&tag_set("allEvents")), [].iter().collect());
        assert_eq!(get_app_id(&tag_set("'a'")), [].iter().collect());
        assert_eq!(
            get_app_id(&tag_set("appId(a)")),
            [ScopedTag::internal(tag!("app_id:a"))].iter().collect()
        );
        assert_eq!(
            get_app_id(&tag_set("appId(a) & appId(b)")),
            [
                ScopedTag::internal(tag!("app_id:a")),
                ScopedTag::internal(tag!("app_id:b"))
            ]
            .iter()
            .collect()
        );
    }

    #[test]
    fn tags() {
        assert_eq!(get_tags(&tag_set("allEvents")), [].iter().collect());
        assert_eq!(get_tags(&tag_set("'a'")), [ScopedTag::app(tag!("a"))].iter().collect());
        assert_eq!(
            get_tags(&tag_set("'a' & 'b'")),
            [ScopedTag::app(tag!("a")), ScopedTag::app(tag!("b"))].iter().collect()
        );
    }

    #[test]
    fn scoped_tags() {
        assert_eq!(get_scoped_tags(&tag_set("allEvents")), Some([].iter().collect()));
        assert_eq!(
            get_scoped_tags(&tag_set("'a' & 'b' & 'a' & appId(c) & appId(c)")),
            Some(
                [
                    ScopedTag::app(tag!("a")),
                    ScopedTag::app(tag!("b")),
                    ScopedTag::internal(tag!("app_id:c"))
                ]
                .iter()
                .collect()
            )
        );
        assert_eq!(get_scoped_tags(&tag_set("'a' & 'b' & appId(c) & appId(d)")), None);
    }

    fn tq(s: &str) -> TimeQuery {
        let mut q = None;
        get_time_query(&tag_set(s), &mut q).unwrap();
        q.unwrap()
    }

    #[test]
    fn time_query() {
        assert_eq!(tq("allEvents"), TimeQuery::all());
        assert_eq!(tq("from(12)"), TimeQuery::all());
        assert_eq!(
            tq("from(2021-01-01Z)"),
            TimeQuery::from(Timestamp::new(1_609_459_200_000_000)..)
        );
        assert_eq!(
            tq("to(2021-01-01Z)"),
            TimeQuery::from(..Timestamp::new(1_609_459_200_000_000))
        );
        assert_eq!(
            tq("from(2021-01-01Z) & to(2021-01-02Z)"),
            TimeQuery::from(Timestamp::new(1_609_459_200_000_000)..Timestamp::new(1_609_545_600_000_000))
        );
        assert_eq!(tq("from(2021-01-01Z) & to(2021-01-01Z)"), TimeQuery::empty());
    }

    fn lq(s: &str) -> LamportQueryBuilder {
        let mut q = None;
        get_lamport_query(&tag_set(s), &mut q).unwrap();
        q.unwrap()
    }

    #[test]
    fn lamport_query() {
        assert_eq!(lq("allEvents"), LamportQueryBuilder::all());
        assert_eq!(lq("from(2021-01-01Z)"), LamportQueryBuilder::all());
        assert_eq!(
            lq("from(1)"),
            LamportQueryBuilder::from(SortKey::new(1.into(), StreamId::min())..)
        );
        assert_eq!(
            lq("to(4)"),
            LamportQueryBuilder::from(..SortKey::new(4.into(), StreamId::min()))
        );
        assert_eq!(
            lq("from(1) & to(4)"),
            LamportQueryBuilder::from(SortKey::new(1.into(), StreamId::min())..SortKey::new(4.into(), StreamId::min()))
        );
        assert_eq!(lq("from(1) & to(1)"), LamportQueryBuilder::empty());

        let zero = NodeId::from_bytes(&[0; 32]).unwrap().stream(0.into());
        let one = NodeId::from_bytes(&[1; 32]).unwrap().stream(0.into());
        assert_eq!(
            lq(&format!("from(1/{}) & to(1/{})", zero, one)),
            LamportQueryBuilder::from(SortKey::new(1.into(), zero)..SortKey::new(1.into(), one))
        );
        assert_eq!(
            lq(&format!("from(1/{}) & to(1/{})", one, zero)),
            LamportQueryBuilder::empty()
        );
    }
}
