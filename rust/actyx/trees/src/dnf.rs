use std::collections::BTreeSet;

use actyx_sdk::language::{self, TagAtom};

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
                r.extend(a.iter().filter(|a| **a != TagAtom::AllEvents).cloned());
                r.extend(b.iter().filter(|a| **a != TagAtom::AllEvents).cloned());
                if r.is_empty() {
                    r.insert(TagAtom::AllEvents);
                }
                Self::insert_unless_redundant(&mut ret, r);
            }
        }
        Dnf(ret)
    }
    fn insert_unless_redundant(aa: &mut BTreeSet<BTreeSet<language::TagAtom>>, b: BTreeSet<language::TagAtom>) {
        let mut to_remove = vec![];
        for a in aa.iter() {
            if a.iter().next() == Some(&TagAtom::AllEvents) || a.is_subset(&b) {
                // a is larger than b. E.g. x | x&y
                // keep a, b is redundant
                return;
            } else if b.iter().next() == Some(&TagAtom::AllEvents) || a.is_superset(&b) {
                // a is smaller than b, E.g. x&y | x
                // remove a, keep b
                to_remove.push(a.clone());
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
        dnf(tag_expr)
    }
}

#[cfg(test)]
mod tests {
    use actyx_sdk::{
        language::{TagAtom, TagExpr},
        Tag,
    };

    use super::*;
    use std::str::FromStr;

    fn l(x: &'static str) -> TagExpr {
        TagExpr::Atom(atom(x))
    }

    fn atom(x: &'static str) -> TagAtom {
        TagAtom::Tag(Tag::from_str(x).unwrap())
    }

    fn assert_dnf(expr: TagExpr, dnf: &'static [&'static [&'static str]]) {
        let expected = Dnf(dnf.iter().map(|conj| conj.iter().map(|c| atom(c)).collect()).collect());
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
