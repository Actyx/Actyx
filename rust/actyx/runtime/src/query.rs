use crate::{
    eval::Context,
    operation::{Filter, Operation, Select},
    value::Value,
};
use actyx_sdk::language;

pub struct Query {
    #[allow(dead_code)]
    from: language::TagExpr,
    stages: Vec<Operation>,
}

impl Query {
    pub fn feed(&self, input: Value) -> Vec<Value> {
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

impl From<language::Query> for Query {
    fn from(q: language::Query) -> Self {
        let mut stages = vec![];
        for op in &q.ops {
            match op {
                language::Operation::Filter(f) => stages.push(Operation::Filter(Filter::new(f.clone()))),
                language::Operation::Select(s) => stages.push(Operation::Select(Select::new(s.clone()))),
            }
        }
        Self { from: q.from, stages }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyx_sdk::EventKey;
    use cbor_data::Encoder;

    #[test]
    fn query() {
        let query_str = "FROM 'a' & isLocal FILTER _ < 3 SELECT _ + 2";
        let q = Query::from(query_str.parse::<language::Query>().unwrap());
        let v = Value::new(EventKey::default(), |b| b.encode_u64(3));
        assert_eq!(q.feed(v), vec![]);

        let v = Value::new(EventKey::default(), |b| b.encode_u64(2));
        let r = Value::new(EventKey::default(), |b| b.encode_u64(4));
        assert_eq!(q.feed(v), vec![r]);
    }
}
