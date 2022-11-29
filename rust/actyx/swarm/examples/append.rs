//! This example exists for the purpose of optimizing append performance
//!
//! Can be run using `RUST_LOG=debug cargo run --release --example append to see timings`,
//! or `cargo flamegraph --example append` to see a flamegraph (on a proper linux).
use acto::ActoRef;
use actyx_sdk::{app_id, tags, AppId, Payload};
use std::{path::PathBuf, time::Instant};
use swarm::*;

fn app_id() -> AppId {
    app_id!("test")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    util::setup_logger();
    let dir = tempfile::tempdir()?;
    let db = PathBuf::from(dir.path().join("db").to_str().expect("illegal filename"));
    let index = PathBuf::from(dir.path().join("index").to_str().expect("illegal filename"));
    let config = SwarmConfig {
        index_store: Some(index),
        node_name: Some("append_bench".to_owned()),
        db_path: Some(db),
        ..SwarmConfig::basic()
    };
    let store = BanyanStore::new(config, ActoRef::blackhole()).await?;
    let n: usize = 1000;
    let t0 = Instant::now();
    for (i, tags, payload) in (0..n).map(|i| (i, tags!("abc"), Payload::null())) {
        store.append(0.into(), app_id(), vec![(tags, payload)]).await?;
        println!("{} {}", i, t0.elapsed().as_millis());
    }
    println!("{:?}", dir);
    Ok(())
}
