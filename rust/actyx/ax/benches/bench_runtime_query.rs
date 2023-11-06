use actyx_sdk::{app_id, language};
use cbor_data::Encoder;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use futures::executor::block_on;
use runtime::{
    eval::{Context, RootContext},
    query::Query,
    value::Value,
};
use swarm::event_store_ref::EventStoreRef;

fn store() -> EventStoreRef {
    EventStoreRef::new(|_x| Err(swarm::event_store_ref::Error::Aborted))
}

fn v() -> (Value, RootContext) {
    let cx = Context::new(store());
    let v = Value::synthetic(cx.child().mk_cbor(|b| {
        b.encode_dict(|b| {
            b.with_key("x", |b| b.encode_u64(5));
            b.with_key("y", |b| b.encode_str("hello"));
            b.with_key("z", |b| b.encode_f64(12.34));
        })
    }));
    (v, cx)
}

const QUERY: &str = "FROM allEvents FILTER _.x > 3 | _.y = 'hello' SELECT [_.x + _.z * 3, { one: 1 two: _.y }] AGGREGATE [SUM(_[0]), LAST(_[1].two)]";

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("nnop", |b| {
        let mut query = Query::from(
            language::Query::parse("FROM allEvents").unwrap(),
            app_id!("com.actyx.test"),
        )
        .0
        .make_feeder();
        let (value, cx) = v();
        let cx = cx.child();
        b.iter(|| black_box(block_on(query.feed(Some(value.clone()), &cx))));
    });
    c.bench_function("feed value", |b| {
        let mut query = Query::from(language::Query::parse(QUERY).unwrap(), app_id!("com.actyx.test"))
            .0
            .make_feeder();
        let (value, cx) = v();
        let cx = cx.child();
        b.iter(|| black_box(block_on(query.feed(Some(value.clone()), &cx))));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
