use actyxos_sdk::{language, tags, TagSet};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    ops::{Deref, DerefMut},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
/// One particular intersection of tags selected for subscription.
/// Setting the local flag selects only sources from the local node.
#[serde(rename_all = "camelCase")]
pub struct TagSubscription {
    pub tags: TagSet,
    pub local: bool,
}
impl TagSubscription {
    pub fn new(tags: TagSet) -> Self {
        Self { tags, local: false }
    }
    pub fn local(mut self) -> Self {
        self.local = true;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct TagSubscriptions(Vec<TagSubscription>);
impl TagSubscriptions {
    pub fn all() -> Self {
        // Empty set is the subset of all sets
        Self(vec![TagSubscription::new(tags!())])
    }
    pub fn empty() -> Self {
        Self(vec![])
    }
    pub fn as_tag_sets(&self, is_local: bool) -> Vec<TagSet> {
        self.0
            .iter()
            // If is_local ist set, we can just use all subscriptions. Otherwise, only non-local subscriptions.
            .filter(|x| is_local || !x.local)
            .cloned()
            .map(|x| x.tags)
            .collect()
    }
    pub fn new(s: Vec<TagSubscription>) -> Self {
        Self(s)
    }
    pub fn only_local(&self) -> bool {
        !self.0.is_empty() && self.0.iter().all(|x| x.local)
    }
}

impl From<Dnf> for TagSubscriptions {
    fn from(dnf: Dnf) -> Self {
        let ret = dnf
            .0
            .into_iter()
            .map(|atoms| {
                let mut tags = TagSubscription::new(TagSet::empty());
                for a in atoms {
                    match a {
                        language::TagAtom::Tag(tag) => tags.tags.insert(tag),
                        language::TagAtom::AllEvents => {}
                        language::TagAtom::IsLocal => tags.local = true,
                        language::TagAtom::FromTime(_) => {}
                        language::TagAtom::ToTime(_) => {}
                        language::TagAtom::FromLamport(_) => {}
                        language::TagAtom::ToLamport(_) => {}
                        language::TagAtom::AppId(_) => {}
                    }
                }
                tags
            })
            .collect::<Vec<_>>();
        Self::new(ret)
    }
}
impl From<language::Expression> for TagSubscriptions {
    fn from(e: language::Expression) -> Self {
        match &e {
            language::Expression::Simple(_) => TagSubscriptions::empty(),
            language::Expression::Query(q) => q.into(),
        }
    }
}
impl From<&language::Query> for TagSubscriptions {
    fn from(query: &language::Query) -> Self {
        let dnf: Dnf = (&query.from).into();
        dnf.into()
    }
}
impl From<&language::TagExpr> for TagSubscriptions {
    fn from(tag_expr: &language::TagExpr) -> Self {
        let dnf: Dnf = tag_expr.into();
        dnf.into()
    }
}
impl Deref for TagSubscriptions {
    type Target = Vec<TagSubscription>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for TagSubscriptions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<TagSubscriptions> for Vec<TagSet> {
    fn from(ts: TagSubscriptions) -> Vec<TagSet> {
        ts.0.into_iter().map(|x| x.tags).collect()
    }
}

// invariant: none of the sets are ever empty
#[derive(Debug)]
pub(crate) struct Dnf(pub BTreeSet<BTreeSet<language::TagAtom>>);

impl Dnf {
    pub fn or(self, other: Dnf) -> Self {
        let mut o = self.0;
        o.extend(other.0);
        Dnf(o)
    }

    pub fn and(self, other: Dnf) -> Self {
        let mut ret = BTreeSet::new();
        for a in self.0 {
            for b in &other.0 {
                ret.insert(a.union(b).cloned().collect());
            }
        }
        Dnf(ret)
    }
}

impl From<&language::TagAtom> for Dnf {
    fn from(a: &language::TagAtom) -> Self {
        let mut s = BTreeSet::new();
        s.insert(a.clone());
        let mut s2 = BTreeSet::new();
        s2.insert(s);
        Self(s2)
    }
}

impl From<&language::TagExpr> for Dnf {
    fn from(tag_expr: &language::TagExpr) -> Self {
        fn dnf(expr: &language::TagExpr) -> Dnf {
            match expr {
                language::TagExpr::Or(o) => dnf(&o.0).or(dnf(&o.1)),
                language::TagExpr::And(a) => dnf(&a.0).and(dnf(&a.1)),
                language::TagExpr::Atom(a) => a.into(),
            }
        }
        dnf(&tag_expr)
    }
}
