use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crypto::{KeyStore, KeyStoreRef, SignedMessage};
use parking_lot::RwLock;
use std::sync::Arc;

fn sign(store: KeyStoreRef, n: usize) -> anyhow::Result<SignedMessage> {
    let store = store.read();
    let keys = store.get_pairs();
    let msg = vec![0u8; n];
    store.sign(&msg, keys.keys().cloned())
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut store = KeyStore::default();
    let _ = store.generate_key_pair().unwrap();
    let store = Arc::new(RwLock::new(store));
    c.bench_function("sign 128", |b| b.iter(|| sign(black_box(store.clone()), 128)));
    c.bench_function("sign 1024", |b| b.iter(|| sign(black_box(store.clone()), 1024)));
    c.bench_function("sign 1024*128", |b| {
        b.iter(|| sign(black_box(store.clone()), 1024 * 128))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
