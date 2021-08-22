use actyx_sdk::{language, OffsetMap};
use cbor_data::Encoder;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use futures::executor::block_on;
use runtime::{eval::Context, query::Query, value::Value};
use swarm::event_store_ref::EventStoreRef;

fn store() -> EventStoreRef {
    EventStoreRef::new(|_x| Err(swarm::event_store_ref::Error::Aborted))
}

fn v() -> Value {
    let cx = Context::owned(Default::default(), store(), OffsetMap::empty(), OffsetMap::empty());
    cx.value(|b| {
        b.encode_dict(|b| {
            b.with_key("x", |b| b.encode_u64(5));
            b.with_key("y", |b| b.encode_str("hello"));
            b.with_key("z", |b| b.encode_f64(12.34));
        })
    })
}

const QUERY: &str = "FROM allEvents FILTER _.x > 3 | _.y = 'hello' SELECT [_.x + _.z * 3, { one: 1 two: _.y }] AGGREGATE [SUM(_[0]), LAST(_[1].two)]";

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("nnop", |b| {
        let mut query = Query::from("FROM allEvents".parse::<language::Query>().unwrap()).make_feeder(
            store(),
            OffsetMap::empty(),
            OffsetMap::empty(),
        );
        let value = v();
        b.iter(|| black_box(block_on(query.feed(Some(value.clone())))));
    });
    c.bench_function("feed value", |b| {
        let mut query = Query::from(QUERY.parse::<language::Query>().unwrap()).make_feeder(
            store(),
            OffsetMap::empty(),
            OffsetMap::empty(),
        );
        let value = v();
        b.iter(|| black_box(block_on(query.feed(Some(value.clone())))));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
