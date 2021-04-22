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
impl From<&language::Query> for TagSubscriptions {
    fn from(query: &language::Query) -> Self {
        Dnf::from(&query.from).into()
    }
}
impl From<&language::TagExpr> for TagSubscriptions {
    fn from(tag_expr: &language::TagExpr) -> Self {
        Dnf::from(tag_expr).into()
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
#[derive(Debug, PartialEq)]
pub(crate) struct Dnf(pub BTreeSet<BTreeSet<language::TagAtom>>);

impl Dnf {
    pub fn or(self, other: Dnf) -> Self {
        let mut ret = self.0;
        for b in other.0 {
            Self::insert_unless_redundant(&mut ret, b);
        }
        Dnf(ret)
    }

    pub fn and(self, other: Dnf) -> Self {
        let mut ret = BTreeSet::new();
        for a in self.0 {
            for b in &other.0 {
                let mut r = BTreeSet::new();
                r.extend(a.iter().cloned());
                r.extend(b.iter().cloned());
                Self::insert_unless_redundant(&mut ret, r);
            }
        }
        Dnf(ret)
    }
    fn insert_unless_redundant(aa: &mut BTreeSet<BTreeSet<language::TagAtom>>, b: BTreeSet<language::TagAtom>) {
        let mut to_remove = BTreeSet::new();
        for a in aa.iter() {
            if a.is_subset(&b) {
                // a is larger than b. E.g. x | x&y
                // keep a, b is redundant
                return;
            } else if a.is_superset(&b) {
                // a is smaller than b, E.g. x&y | x
                // remove a, keep b
                to_remove.insert(a.clone());
            }
        }
        for r in to_remove {
            aa.remove(&r);
        }
        aa.insert(b);
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

#[cfg(test)]
mod tests {
    use actyxos_sdk::{
        language::{TagAtom, TagExpr},
        Tag,
    };

    use super::*;

    fn l(x: &'static str) -> TagExpr {
        TagExpr::Atom(atom(x))
    }

    fn atom(x: &'static str) -> TagAtom {
        TagAtom::Tag(Tag::new(x.to_owned()).unwrap())
    }

    fn assert_dnf(expr: TagExpr, dnf: &'static [&'static [&'static str]]) {
        let expected = Dnf(dnf.iter().map(|conj| conj.iter().map(|c| atom(*c)).collect()).collect());
        assert_eq!(Dnf::from(&expr), expected);
    }

    #[test]
    fn test_dnf_intersection_1() {
        let a = l("a");
        let b = l("b");
        let c = l("c");
        assert_dnf(c & (a | b), &[&["a", "c"], &["b", "c"]]);
    }

    #[test]
    fn test_dnf_intersection_2() {
        let a = l("a");
        let b = l("b");
        let c = l("c");
        let d = l("d");
        assert_dnf((d | c) & (b | a), &[&["a", "c"], &["a", "d"], &["b", "c"], &["b", "d"]]);
    }

    #[test]
    fn test_dnf_simplify_1() {
        let a = l("a");
        let b = l("b");
        assert_dnf((a.clone() | b) & a, &[&["a"]]);
    }

    #[test]
    fn test_dnf_simplify_2() {
        let a = l("a");
        let b = l("b");
        assert_dnf((a.clone() & b) | a, &[&["a"]]);
    }

    #[test]
    fn test_dnf_simplify_3() {
        let a = l("a");
        let b = l("b");
        assert_dnf((a.clone() | b) | a, &[&["a"], &["b"]]);
    }

    #[test]
    fn test_dnf_simplify_4() {
        let a = l("a");
        let b = l("b");
        let c = l("c");
        assert_dnf((a.clone() & b).or(a.clone() & c).or(a), &[&["a"]]);
    }
}
