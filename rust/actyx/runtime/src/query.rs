use crate::{
    eval::Context,
    operation::{Filter, Operation, Select},
    value::Value,
};
use actyxos_sdk::{
    language::{Expression, TagAtom, TagExpr},
    EventKey, TagSet,
};
use cbor_data::Encoder;
use std::collections::BTreeSet;
use trees::{TagSubscription, TagSubscriptions};

pub struct Query {
    expr: Expression,
    stages: Vec<Operation>,
}

impl Query {
    pub fn new(expr: Expression) -> Self {
        let mut stages = vec![];
        if let Expression::Query(q) = &expr {
            for op in &q.ops {
                match op {
                    actyxos_sdk::language::Operation::Filter(f) => {
                        stages.push(Operation::Filter(Filter::init(f.clone())))
                    }
                    actyxos_sdk::language::Operation::Select(s) => {
                        stages.push(Operation::Select(Select::init(s.clone())))
                    }
                }
            }
        }
        Self { expr, stages }
    }

    pub fn event_selection(&self) -> TagSubscriptions {
        let query = match &self.expr {
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

    pub fn initial_result(&self) -> Vec<Value> {
        match &self.expr {
            Expression::Simple(expr) => {
                let ret = Context::new(EventKey::default())
                    .eval(expr)
                    .unwrap_or_else(|err| Value::new(EventKey::default(), |b| b.encode_str(&*err.to_string())));
                vec![ret]
            }
            Expression::Query(_) => vec![],
        }
    }

    pub fn feed(&mut self, input: Value) -> Vec<Value> {
        fn rec<'a>(cx: &'a Context, input: Value, mut ops: impl Iterator<Item = &'a Operation> + Clone) -> Vec<Value> {
            if let Some(op) = ops.next() {
                let (vs, cx) = op.apply(cx, input);
                vs.into_iter().flat_map(|v| rec(cx, v, ops.clone())).collect()
            } else {
                vec![input]
            }
        }
        rec(&Context::new(input.sort_key), input, self.stages.iter())
    }
}

// invariant: none of the sets are ever empty
struct Dnf(BTreeSet<BTreeSet<TagAtom>>);
impl From<&TagAtom> for Dnf {
    fn from(a: &TagAtom) -> Self {
        let mut s = BTreeSet::new();
        s.insert(a.clone());
        let mut s2 = BTreeSet::new();
        s2.insert(s);
        Self(s2)
    }
}
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

impl From<Dnf> for TagSubscriptions {
    fn from(dnf: Dnf) -> Self {
        let ret = dnf
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
    use actyxos_sdk::{language::expression, tags, EventKey};
    use cbor_data::Encoder;

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

    #[test]
    fn empty() {
        let e = expression("42").unwrap();
        let q = Query::new(e);
        assert_eq!(q.event_selection(), TagSubscriptions::empty())
    }

    #[test]
    fn query() {
        let mut q = Query::new(expression("FROM 'a' & isLocal FILTER _ < 3 SELECT _ + 2").unwrap());
        assert_eq!(
            q.event_selection(),
            TagSubscriptions::new(vec![TagSubscription::new(tags!("a")).local()])
        );

        let v = Value::new(EventKey::default(), |b| b.encode_u64(3));
        assert_eq!(q.feed(v), vec![]);

        let v = Value::new(EventKey::default(), |b| b.encode_u64(2));
        let r = Value::new(EventKey::default(), |b| b.encode_u64(4));
        assert_eq!(q.feed(v), vec![r]);
    }
}
