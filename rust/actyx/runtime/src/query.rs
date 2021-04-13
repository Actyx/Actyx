use crate::{
    eval::Context,
    operation::{Filter, Operation, Select},
    value::Value,
};
use actyxos_sdk::{language, EventKey, TagSet};
use cbor_data::Encoder;
use std::collections::BTreeSet;
use trees::{TagSubscription, TagSubscriptions};

pub struct Query {
    expr: language::Expression,
    stages: Vec<Operation>,
}

impl Query {
    pub fn new(expr: language::Expression) -> Self {
        let mut stages = vec![];
        if let language::Expression::Query(q) = &expr {
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

    pub fn initial_result(&self) -> Vec<Value> {
        match &self.expr {
            language::Expression::Simple(expr) => {
                let ret = Context::new(EventKey::default())
                    .eval(expr)
                    .unwrap_or_else(|err| Value::new(EventKey::default(), |b| b.encode_str(&*err.to_string())));
                vec![ret]
            }
            language::Expression::Query(_) => vec![],
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
struct Dnf(BTreeSet<BTreeSet<language::TagAtom>>);
impl From<&language::TagAtom> for Dnf {
    fn from(a: &language::TagAtom) -> Self {
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

impl From<&language::Query> for Dnf {
    fn from(q: &language::Query) -> Self {
        fn dnf(expr: &language::TagExpr) -> Dnf {
            match expr {
                language::TagExpr::Or(o) => dnf(&o.0).or(dnf(&o.1)),
                language::TagExpr::And(a) => dnf(&a.0).and(dnf(&a.1)),
                language::TagExpr::Atom(a) => a.into(),
            }
        }
        dnf(&q.from)
    }
}

impl From<&Query> for TagSubscriptions {
    fn from(q: &Query) -> Self {
        match &q.expr {
            language::Expression::Simple(_) => TagSubscriptions::empty(),
            language::Expression::Query(q) => {
                let dnf: Dnf = q.into();
                dnf.into()
            }
        }
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
        TagSubscriptions::new(ret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyxos_sdk::{language::expression, tags, EventKey};
    use cbor_data::Encoder;

    fn test_query(expr: &'static str, tag_subscriptions: TagSubscriptions) {
        let e = expression(expr).unwrap();
        let q = &Query::new(e);
        let s: TagSubscriptions = (q).into();
        assert_eq!(s, tag_subscriptions);
    }

    #[test]
    fn parsing() {
        test_query(
            "FROM 'a' & isLocal | ('b' | 'c') & allEvents & 'd'",
            TagSubscriptions::new(vec![
                TagSubscription::new(tags!("a")).local(),
                TagSubscription::new(tags!("b", "d")),
                TagSubscription::new(tags!("c", "d")),
            ]),
        );
    }

    #[test]
    fn all_events() {
        test_query("FROM allEvents", TagSubscriptions::all());
    }

    #[test]
    fn empty() {
        test_query("42", TagSubscriptions::empty())
    }

    #[test]
    fn query() {
        let expr = "FROM 'a' & isLocal FILTER _ < 3 SELECT _ + 2";
        test_query(
            expr,
            TagSubscriptions::new(vec![TagSubscription::new(tags!("a")).local()]),
        );

        let mut q = Query::new(expression(expr).unwrap());
        let v = Value::new(EventKey::default(), |b| b.encode_u64(3));
        assert_eq!(q.feed(v), vec![]);

        let v = Value::new(EventKey::default(), |b| b.encode_u64(2));
        let r = Value::new(EventKey::default(), |b| b.encode_u64(4));
        assert_eq!(q.feed(v), vec![r]);
    }
}
