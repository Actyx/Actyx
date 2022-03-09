use actyx_sdk::app_id;
use std::{fs::File, os::unix::prelude::FileExt, time::Instant};
use swarm::{blob_store::BlobStore, DbPath};

const SIZE: usize = 18;
const COUNT: usize = 900;
const SLOTS: usize = COUNT - 1;

fn make_blob() -> anyhow::Result<Vec<u8>> {
    let mut v = Vec::new();
    v.resize(SIZE, 0u8);
    let rand = File::open("/dev/random")?;
    rand.read_exact_at(v.as_mut_slice(), 0)?;
    Ok(v)
}

fn main() {
    let store = BlobStore::new(DbPath::File("blob".into())).unwrap();
    let blobs = (0..COUNT)
        .map(|_| make_blob())
        .collect::<anyhow::Result<Vec<_>>>()
        .unwrap();
    println!("blobs generated");
    let id = app_id!("me");
    let mime = "application/octet-stream".to_owned();
    let start = Instant::now();
    for (idx, blob) in blobs.iter().cycle().enumerate() {
        let name = format!("{}", idx % SLOTS);
        store.blob_put(id.clone(), name, mime.clone(), blob.as_slice()).unwrap();
        if idx > 0 && idx & 255 == 0 {
            let time = start.elapsed();
            println!("put {} in {:?}, {}µs", idx, time, time.as_micros() / (idx as u128));
        }
    }
}
