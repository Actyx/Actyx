use actyxos_sdk::{
    language::{Expression, TagAtom, TagExpr},
    tagged::TagSet,
};
use trees::{TagSubscription, TagSubscriptions};

pub struct Query(Expression);

impl Query {
    pub fn new(expr: Expression) -> Self {
        Self(expr)
    }

    pub fn event_selection(&self) -> TagSubscriptions {
        let query = match &self.0 {
            Expression::Simple(_) => return TagSubscriptions::empty(),
            Expression::Query(q) => q,
        };

        fn dnf(expr: &TagExpr) -> Dnf {
            match expr {
                TagExpr::Or(o) => dnf(&o.0).or(dnf(&o.1)),
                TagExpr::And(a) => dnf(&a.0).and(dnf(&a.1)),
                TagExpr::Atom(a) => a.into(),
            }
        }

        dnf(&query.from).into()
    }
}

struct Dnf(Vec<Vec<TagAtom>>);
impl From<&TagAtom> for Dnf {
    fn from(a: &TagAtom) -> Self {
        Self(vec![vec![a.clone()]])
    }
}
impl Dnf {
    pub fn or(self, other: Dnf) -> Self {
        let mut o = self.0;
        o.extend(other.0);
        Dnf(o)
    }
    pub fn and(self, other: Dnf) -> Self {
        if other.0.is_empty() {
            return self;
        }
        let mut ret = vec![];
        for mut a in self.0 {
            let last = other.0.len() - 1;
            for b in &other.0[0..last] {
                ret.push(a.iter().chain(b.iter()).cloned().collect());
            }
            a.extend(other.0[last].clone());
            ret.push(a);
        }
        Dnf(ret)
    }
}
impl Into<TagSubscriptions> for Dnf {
    fn into(self) -> TagSubscriptions {
        let ret = self
            .0
            .into_iter()
            .map(|atoms| {
                let mut tags = TagSubscription::new(TagSet::empty());
                for a in atoms {
                    match a {
                        TagAtom::Tag(tag) => tags.tags.insert(tag),
                        TagAtom::AllEvents => {}
                        TagAtom::IsLocal => tags.local = true,
                        TagAtom::FromTime(_) => {}
                        TagAtom::ToTime(_) => {}
                        TagAtom::FromLamport(_) => {}
                        TagAtom::ToLamport(_) => {}
                        TagAtom::AppId(_) => {}
                    }
                }
                tags
            })
            .collect::<Vec<_>>();
        TagSubscriptions::new(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyxos_sdk::{language::expression, tags};

    #[test]
    fn parsing() {
        let e = expression("FROM 'a' & isLocal | ('b' | 'c') & allEvents & 'd'").unwrap();
        let q = Query::new(e);
        assert_eq!(
            q.event_selection(),
            TagSubscriptions::new(vec![
                TagSubscription::new(tags!("a")).local(),
                TagSubscription::new(tags!("b", "d")),
                TagSubscription::new(tags!("c", "d")),
            ])
        );
    }

    #[test]
    fn all_events() {
        let e = expression("FROM allEvents").unwrap();
        let q = Query::new(e);
        assert_eq!(q.event_selection(), TagSubscriptions::all());
    }
}
