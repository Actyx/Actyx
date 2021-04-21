use std::{
    path::Path,
    time::{Duration, Instant},
};

use actyxos_sdk::{app_id, AppManifest, HttpClient};
use crossbeam::channel::Receiver;
use node::{ApplicationState, BindTo};
use url::Url;
use util::formats::LogRequest;

fn get_eventservice_port(rx: &Receiver<LogRequest>) -> anyhow::Result<u16> {
    let start = Instant::now();
    let regex = regex::Regex::new(r"(?:API bound to 127.0.0.1:)(\d*)").unwrap();

    loop {
        let ev = rx.recv_deadline(start.checked_add(Duration::from_millis(5000)).unwrap())?;
        if let Some(x) = regex.captures(&*ev.message).and_then(|c| c.get(1).map(|x| x.as_str())) {
            return Ok(x.parse()?);
        }
    }
}

pub async fn mk_http_client(port: u16) -> anyhow::Result<HttpClient> {
    let app_manifest = AppManifest::new(
        app_id!("com.example.trial-mode"),
        "display name".into(),
        "0.1.0".into(),
        None,
    );
    let url = Url::parse(&format!("http://localhost:{}", port)).unwrap();
    HttpClient::new(url, app_manifest).await
}

pub async fn start_node(
    path: impl AsRef<Path>,
    logging_rx: &Receiver<LogRequest>,
) -> anyhow::Result<(ApplicationState, HttpClient)> {
    let node = ApplicationState::spawn(
        path.as_ref().into(),
        node::Runtime::Linux,
        BindTo {
            api: "localhost:0".parse().unwrap(),
            ..BindTo::random()
        },
    )?;
    let port = get_eventservice_port(logging_rx)?;
    let es = mk_http_client(port).await?;

    Ok((node, es))
}
