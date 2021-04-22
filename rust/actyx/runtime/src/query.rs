use crate::{
    eval::Context,
    operation::{Filter, Operation, Select},
    value::Value,
};
use actyxos_sdk::{language, EventKey};
use cbor_data::Encoder;

pub enum Query {
    Expr(language::SimpleExpr),
    Query(Vec<Operation>),
}

impl Query {
    pub fn initial_result(&self) -> Vec<Value> {
        match &self {
            Query::Expr(expr) => {
                let ret = Context::new(EventKey::default())
                    .eval(expr)
                    .unwrap_or_else(|err| Value::new(EventKey::default(), |b| b.encode_str(&*err.to_string())));
                vec![ret]
            }
            Query::Query(_) => vec![],
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
        match self {
            Query::Query(stages) => rec(&Context::new(input.sort_key), input, stages.iter()),
            Query::Expr(_) => vec![input],
        }
    }
}

impl From<&language::Expression> for Query {
    fn from(expr: &language::Expression) -> Self {
        match expr {
            language::Expression::Query(q) => q.into(),
            language::Expression::Simple(s) => Self::Expr(s.clone()),
        }
    }
}

impl From<&language::Query> for Query {
    fn from(q: &language::Query) -> Self {
        let mut stages = vec![];
        for op in &q.ops {
            match op {
                actyxos_sdk::language::Operation::Filter(f) => stages.push(Operation::Filter(Filter::new(f.clone()))),
                actyxos_sdk::language::Operation::Select(s) => stages.push(Operation::Select(Select::new(s.clone()))),
            }
        }
        Self::Query(stages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyxos_sdk::{language::Expression, tags, EventKey};
    use cbor_data::Encoder;
    use trees::{TagSubscription, TagSubscriptions};

    fn test_query(expr: &'static str, tag_subscriptions: TagSubscriptions) {
        let e: Expression = expr.parse().unwrap();
        assert_eq!(TagSubscriptions::from(e), tag_subscriptions);
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

        let mut q = Query::from(&expr.parse::<Expression>().unwrap());
        let v = Value::new(EventKey::default(), |b| b.encode_u64(3));
        assert_eq!(q.feed(v), vec![]);

        let v = Value::new(EventKey::default(), |b| b.encode_u64(2));
        let r = Value::new(EventKey::default(), |b| b.encode_u64(4));
        assert_eq!(q.feed(v), vec![r]);
    }
}
