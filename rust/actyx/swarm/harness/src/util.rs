use actyxos_sdk::{app_id, service::EventService, AppManifest, HttpClient, OffsetMap, Url};
use futures::{stream::FuturesOrdered, Future, StreamExt};
use quickcheck::TestResult;
use std::net::SocketAddrV4;

use crate::{run_netsim_quickcheck, HarnessOpts};

// Returns HttpClients in the same order
pub async fn mk_clients(apis: Vec<SocketAddrV4>) -> anyhow::Result<Vec<HttpClient>> {
    Ok(apis
        .into_iter()
        .map(mk_client)
        .collect::<FuturesOrdered<_>>()
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<anyhow::Result<_>>()?)
}
pub async fn mk_client(api: SocketAddrV4) -> anyhow::Result<HttpClient> {
    let url = Url::parse(&format!("http://{}", api))?;
    let c = HttpClient::new(url, app_manifest()).await?;
    Ok(c)
}

fn app_manifest() -> AppManifest {
    AppManifest::new(
        app_id!("com.example.trial-mode"),
        "display name".into(),
        "0.1.0".into(),
        None,
    )
}
pub async fn await_stream_offsets(stores: &[HttpClient], target_offsets: &OffsetMap) -> anyhow::Result<()> {
    for store in stores {
        loop {
            let o = store.offsets().await?.present;
            if o >= *target_offsets {
                break;
            }
        }
    }
    Ok(())
}

pub fn run_quickcheck<F, F2>(n_nodes: usize, f: F) -> anyhow::Result<TestResult>
where
    F: FnOnce(Vec<SocketAddrV4>) -> F2 + Send + 'static,
    F2: Future<Output = anyhow::Result<TestResult>>,
{
    let opts = HarnessOpts {
        n_nodes,
        n_bootstrap: 1,
        delay_ms: 0,
        enable_mdns: false,
        enable_fast_path: true,
        enable_slow_path: true,
        enable_root_map: true,
        enable_discovery: true,
        enable_metrics: true,
        enable_api: Some("0.0.0.0:30001".parse().unwrap()),
    };
    let r = std::thread::spawn(move || run_netsim_quickcheck(opts, f))
        .join()
        .map_err(|e| anyhow::anyhow!("Caught panic {:?}", e));
    println!("{:?}", r);
    r?
}
