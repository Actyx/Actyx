use crate::cmd;
use anyhow::Result;
use async_trait::async_trait;
use ax_config::StoreConfig;
use bytes::BytesMut;
use chrono::{DateTime, Utc};
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::Future;
use futures::{future, FutureExt, StreamExt, TryStreamExt};
use hyper::client::connect::dns::GaiResolver;
use hyper::client::HttpConnector;
use hyper::header::HeaderValue;
use hyper::{Body, Client as HyperClient, Method, Request};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::pin::Pin;
use store_core::live::{LiveEvents, Topic};
use store_core::BanyanStore;
use tracing::*;
use trees::monitoring::{FullMonitoringMessage, PublishLog};
use util::serde_util::from_json_or_cbor_slice;
use warp::http::Uri;

pub struct Cmd;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LokiLogEntry {
    ts: DateTime<Utc>,
    line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LokiLogStream {
    labels: String,
    entries: Vec<LokiLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LokiPublishLog {
    streams: Vec<LokiLogStream>,
}

pub fn args() -> App<'static, 'static> {
    SubCommand::with_name("logsToLoki")
        .about("Send logs and distressCalls from pubsub to Loki")
        .arg(
            Arg::with_name("topic")
                .help("MonitoringTopic")
                .long("topic")
                .short("t")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("customer")
                .help("Customer ID")
                .long("customer")
                .short("c")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("loki_uri")
                .help("Loki URI")
                .long("loki")
                .short("l")
                .takes_value(true)
                .required(true),
        )
}

#[async_trait]
impl cmd::Command for Cmd {
    fn name(&self) -> &str {
        "logsToLoki"
    }

    async fn run(&self, matches: &ArgMatches<'_>, _config: StoreConfig, store: BanyanStore) -> Result<()> {
        let topic = matches.value_of("topic").expect("Couldn't parse topic").to_string();
        let customer = matches
            .value_of("customer")
            .expect("Couldn't parse customer")
            .to_string();

        let loki_uri = matches
            .value_of("loki_uri")
            .expect("Couldn't parse Loki URI")
            .to_string();

        run_cmd(std::io::stdout(), store, topic, customer, loki_uri).await;
        Ok(())
    }
}

type BoxedHyperFuture<T> = Pin<Box<dyn Future<Output = Result<T, hyper::Error>> + Send>>;

async fn run_cmd<W>(write: W, store: BanyanStore, topic: String, customer: String, loki_uri: String) -> W
where
    W: Write + Send + 'static,
{
    let hyper_client = HyperClient::new();
    let client = store.ipfs();
    // TODO Change to /loki/api/v1/push and fix the format (see https://github.com/grafana/loki/blob/master/docs/api.md#post-lokiapiv1push)
    let uri: Uri = format!("{}/api/prom/push", loki_uri).parse().unwrap();

    let pipeline: Pin<Box<dyn Future<Output = ()> + Send>> = {
        eprintln!("Streaming logs for customer {} to {}", customer, uri);
        let live = LiveEvents::new(&client);

        // neverending stream, all errors are signalled, but the stream is attempted indefinitely
        Box::pin(
            live.listen_raw(&Topic(topic))
                .unwrap()
                .filter_map(|raw_pubsub_ev| future::ready(decode(raw_pubsub_ev)))
                .map(move |messages| convert_to_loki(&customer, messages))
                .then(move |loki_message| post_to_loki(&hyper_client, &uri, &loki_message))
                .for_each(|_| future::ready(())),
        )
    };

    pipeline.await;
    write
}
fn post_to_loki(
    hyper_client: &HyperClient<HttpConnector<GaiResolver>, Body>,
    uri: &Uri,
    loki_message: &LokiPublishLog,
) -> BoxedHyperFuture<()> {
    serde_json::to_string(&loki_message)
        .map_err(|err| eprintln!("Error while serializing Loki message to JSON: {}", err))
        .and_then(|json| {
            debug!("Posting to Loki: {}", &json);

            Request::builder()
                .method(Method::POST)
                .uri(uri)
                .header(
                    hyper::header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                )
                .body(Body::from(json.clone()))
                .map_err(|err| eprintln!("Error when creating request to POST to Loki {:?}", err))
                .map(|req| (req, json))
        })
        .map(|(req, json)| -> BoxedHyperFuture<()> {
            Box::pin(hyper_client.request(req).then(|resp| print_loki_error(resp, json)))
        })
        .unwrap_or_else(|_| Box::pin(futures::future::ready(Ok(()))))
}

fn print_loki_error(
    response: Result<hyper::Response<Body>, hyper::Error>,
    orig_payload: String,
) -> BoxedHyperFuture<()> {
    match response {
        Ok(resp) => {
            debug!("Posted, status: {}", resp.status());

            if !resp.status().is_success() {
                let buf = BytesMut::new();
                Box::pin(
                    resp.into_body()
                        .try_fold(buf, |mut buf, bytes| async move {
                            buf.extend(&bytes);
                            Ok(buf)
                        })
                        .map(move |chunk| {
                            let v = chunk.expect("Error returned while retrieving loki body").to_vec();
                            let body = String::from_utf8_lossy(&v).to_string();
                            eprintln!("Error when posting to Loki: {}. Payload: {}", body, orig_payload);
                            Ok(())
                        }),
                )
            } else {
                Box::pin(futures::future::ready(Ok(())))
            }
        }
        Err(err) => {
            eprintln!("Error when posting to Loki: {}", err);
            Box::pin(futures::future::ready(Ok(())))
        }
    }
}

fn convert_to_loki(customer: &str, messages: Vec<PublishLog>) -> LokiPublishLog {
    // TODO Handle distressCalls
    let streams = messages
        .into_iter()
        .map(|msg: PublishLog| {
            // without this bizarreness, you get "Temporary value dropped while borrowed"
            let source_id = &msg.source_id().as_str().to_string();
            let level = &msg.level().to_lowercase();
            let standard_labels = vec![
                ("customer".to_string(), customer),
                ("tag".to_string(), msg.tag()),
                ("serialNumber".to_string(), msg.serial_number()),
                ("sourceId".to_string(), source_id),
                ("level".to_string(), level),
            ]
            .into_iter();

            // TODO group by common labels
            let labels_str = standard_labels
                .map(|(key, value)| {
                    format!(
                        "{}=\"{}\"",
                        // TODO check further sanitization
                        key.replace(".", "_").replace("-", "_"),
                        value.replace("\"", "'")
                    )
                })
                .join(",");

            let labels = format!("{{{}}}", labels_str);

            let mut entries = vec![LokiLogEntry {
                // We can't use the message's timestamp because they can come out of order and Loki expects timestamps to be monotonically increasing within a single stream
                ts: Utc::now(),
                line: format!("[{}] {}", msg.timestamp().to_rfc3339(), msg.message()),
            }];

            entries.sort_by_key(|entry| entry.ts);

            LokiLogStream { labels, entries }
        })
        .collect();

    LokiPublishLog { streams }
}

fn decode(raw_pubsub_ev: Vec<u8>) -> Option<Vec<PublishLog>> {
    // We will drop frames that we cannot decode instead of failing. This stream will
    // only fail if the underlying HTTP stream fails.
    from_json_or_cbor_slice::<FullMonitoringMessage>(raw_pubsub_ev.as_slice())
        .map_err(|err| {
            // TODO Handle distressCalls
            debug!(
                "Cannot deserialize as monitoring message. Error: {}, message: {}",
                err,
                String::from_utf8(
                    base64::decode(&raw_pubsub_ev).unwrap_or_else(|err| format!(
                        "> unable to decode < {}",
                        err.to_string()
                    )
                    .into_bytes())
                )
                .unwrap_or_else(|err| err.to_string())
            );
        })
        .ok()
        .and_then(|decoded| {
            debug!("Received message from pubsub: {:?}", decoded);
            match decoded {
                FullMonitoringMessage::Log(msgs) => Some(msgs),
                _ => None,
            }
        })
}
