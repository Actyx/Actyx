//! This example exists for the purpose of optimizing append performance
//!
//! Can be run using `RUST_LOG=debug cargo run --release --example append to see timings`,
//! or `cargo flamegraph --example append` to see a flamegraph (on a proper linux).
use actyx_sdk::{app_id, tags, AppId, Payload};
use parking_lot::Mutex;
use std::{path::PathBuf, sync::Arc, time::Instant};
use swarm::*;
use tempdir::TempDir;
use util::set_log_level;

fn app_id() -> AppId {
    app_id!("test")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    set_log_level(0);
    let dir = TempDir::new("append_bench")?;
    let db = PathBuf::from(dir.path().join("db").to_str().expect("illegal filename"));
    let index = PathBuf::from(dir.path().join("index").to_str().expect("illegal filename"));
    let index_store = Arc::new(Mutex::new(rusqlite::Connection::open(index)?));
    let config = SwarmConfig {
        index_store: Some(index_store),
        node_name: Some("append_bench".to_owned()),
        db_path: Some(db),
        ..SwarmConfig::basic()
    };
    let store = BanyanStore::new(config).await?;
    let n: usize = 1000;
    let t0 = Instant::now();
    for (i, tags, payload) in (0..n).map(|i| (i, tags!("abc"), Payload::empty())) {
        store.append(0.into(), app_id(), vec![(tags, payload)]).await?;
        println!("{} {}", i, t0.elapsed().as_millis());
    }
    println!("{:?}", dir);
    Ok(())
}
