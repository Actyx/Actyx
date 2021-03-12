use crate::cmd;
use actyxos_lib::chunkedresponse::ChunkedResponse;
use actyxos_sdk::event::SourceId;
use anyhow::Result;
use async_trait::async_trait;
use ax_config::StoreConfig;
use chrono::{TimeZone, Utc};
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::{future::FutureExt, Future, SinkExt, Stream, StreamExt, TryStreamExt};
use hyper::{Body, Client as HyperClient, Method, Request};
use ipfs_node::IpfsNode;
use itertools::Itertools;
use lazy_static::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    fs,
    io::{BufRead, BufReader, Write},
    pin::Pin,
    str::FromStr,
};
use swarm::BanyanStore;
use tracing::*;
use trees::{monitoring::FullMonitoringMessage, PublishLog};
use warp::http::{header::CONTENT_TYPE, HeaderValue, Response, Uri};

pub struct Cmd;

// See https://www.freedesktop.org/wiki/Software/systemd/json/
//    and https://github.com/Actyx/Cosmos/issues/1778#issuecomment-529770308
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct JournaldJson {
    container_name: String,
    _source_realtime_timestamp: String,
    message: Vec<u8>,
    container_id_full: String,
}

pub fn args() -> App<'static, 'static> {
    SubCommand::with_name("balenaLogsToPubsub")
        .about("Send Balena logs from a device to the monitoringTopic. This uses the Balena Supervisor API.")
        .arg(
            Arg::with_name("monitoringTopic")
                .help("MonitoringTopic")
                .long("monitoringTopic")
                .short("m")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("balena_supervisor_uri")
                .help("Balena Supervisor URI (default: $BALENA_SUPERVISOR_ADDRESS)")
                .long("supervisor")
                .short("s")
                .takes_value(true)
                .required(true)
                .env("BALENA_SUPERVISOR_ADDRESS"), // Requires balena container labels - see https://www.balena.io/docs/learn/develop/multicontainer/#labels
        )
        .arg(
            Arg::with_name("balena_supervisor_api_key")
                .help("Balena Supervisor API Key (default: $BALENA_SUPERVISOR_API_KEY)")
                .long("api-key")
                .short("k")
                .takes_value(true)
                .required(true)
                .env("BALENA_SUPERVISOR_API_KEY"), // Requires balena container labels - see https://www.balena.io/docs/learn/develop/multicontainer/#labels
        )
        .arg(
            Arg::with_name("serial_number")
                .help("Device serial number (default: $BALENA_DEVICE_UUID)")
                .long("serial")
                .short("n")
                .takes_value(true)
                .required(true)
                .env("BALENA_DEVICE_UUID"),
        )
}

#[async_trait]
impl cmd::Command for Cmd {
    fn name(&self) -> &str {
        "balenaLogsToPubsub"
    }

    async fn run(&self, matches: &ArgMatches<'_>, _config: StoreConfig, store: BanyanStore) -> Result<()> {
        let topic = matches
            .value_of("monitoringTopic")
            .expect("Couldn't parse monitoringTopic")
            .to_string();

        let supervisor_uri: Uri = matches
            .value_of("balena_supervisor_uri")
            .and_then(|param| param.to_string().parse().ok())
            .expect("Couldn't parse Balena Supervisor URI");

        let supervisor_api_key = matches
            .value_of("balena_supervisor_api_key")
            .expect("Couldn't parse Balena token")
            .to_string();

        let serial_number = matches
            .value_of("serial_number")
            .expect("Couldn't parse device serial number")
            .to_string();

        run_cmd(
            std::io::stdout(),
            store,
            topic,
            supervisor_uri,
            supervisor_api_key,
            serial_number,
        )
        .await;
        Ok(())
    }
}

type BoxedHyperFuture<T> = Pin<Box<dyn Future<Output = Result<T, hyper::Error>> + Send>>;

#[allow(clippy::too_many_arguments)]
async fn run_cmd<W>(
    write: W,
    store: BanyanStore,
    topic: String,
    supervisor_address: Uri,
    supervisor_api_key: String,
    serial_number: String,
) -> W
where
    W: Write + Send + 'static,
{
    let hyper_client = HyperClient::new();
    let uri: Uri = format!("{}v2/journal-logs?apikey={}", supervisor_address, supervisor_api_key)
        .parse()
        .expect("Error parsing Balena supervisor endpoint");

    let client = store.ipfs().clone();

    validate_supervisor_version();

    debug!("Balena supervisor endpoint: {}", uri);

    let pipeline: BoxedHyperFuture<()> = {
        info!("Streaming logs for to topic {}", topic);

        let req = create_supervisor_request(uri);
        debug!("Request: {:?}", req);

        Box::pin(hyper_client.request(req).then(move |response| match response {
            Err(err) => {
                error!("Error in request to Balena supervisor: {:?}", err);
                Box::pin(futures::future::ready(Ok(())))
            }
            Ok(resp) => process_supervisor_response(resp, topic, serial_number, client),
        }))
    };

    pipeline.await.unwrap();
    write
}

fn get_container_id() -> String {
    // https://stackoverflow.com/questions/20995351/how-can-i-get-docker-linux-container-information-from-within-the-container-itsel
    // This will get executed right after the container starts. Since it's an internal service, let's just keep the unwraps()
    let file = fs::File::open("/proc/self/cgroup").unwrap();
    let reader = BufReader::new(file);
    let first = reader
        .lines()
        .find(|line| line.as_ref().unwrap().starts_with("0:"))
        .unwrap()
        .unwrap();

    debug!("cgroup line: {}", first);

    // 10:devices:/system.slice/docker-06ba42b06b28733dfbfd23a4e9670dd3051abd7a3c4e3f39b6566881e8700669.scope
    let re = Regex::new(r#"[0-9]+:[^:]*:/[^/]+/[^-]+-([0-9a-z]+)"#).unwrap();

    re.captures(first.as_str())
        .and_then(|captures| captures.get(1))
        .map(|m| {
            info!("Container id: {}", m.as_str());
            m.as_str().to_string()
        })
        .unwrap()
}

fn create_supervisor_request(uri: Uri) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(Body::from(r#"{"follow":true,"all":true,"format":"json"}"#))
        .expect("Error creating request to Balena supervisor")
}

fn validate_supervisor_version() {
    let supervisor_version = std::env::var("BALENA_SUPERVISOR_VERSION");
    match supervisor_version {
        Ok(version) => {
            let ver = version.split('.').map(|v| format!("{:02}", v)).join("");

            // For this to work correctly we need at least supervisor version 10.3.0
            if ver.as_str() < "100300" {
                error!(
                    "Balena supervisor version {} not supported. Please upgrade to supervisor 10.3.0+",
                    version
                );
                std::process::exit(1);
            }
        }

        Err(_) => {
            error!("BALENA_SUPERVISOR_VERSION environment variable not found, bailing out");
            std::process::exit(1);
        }
    }
}

fn process_supervisor_response(
    response: Response<Body>,
    topic: String,
    serial_number: String,
    ipfs: IpfsNode,
) -> BoxedHyperFuture<()> {
    debug!("Supervisor response: {:?}", response);

    if !response.status().is_success() {
        error!("Error response from Balena supervisor: {:?}", response);
        return Box::pin(futures::future::ready(Ok(())));
    }

    let (publish_tx, publish_rx) = futures::channel::mpsc::unbounded::<Vec<u8>>();
    let container_id = get_container_id();

    let publish_future = publish_rx.for_each(move |message| {
        let _ = ipfs.publish(topic.as_str(), message);
        futures::future::ready(())
    });

    tokio::spawn(publish_future);

    Box::pin(
        convert_response(response, serial_number, container_id)
            .forward(publish_tx.sink_map_err(|err| {
                error!("Error writing to channel: {:?}", err);
            }))
            .map(|_| Ok(())),
    )
}

fn convert_response(
    response: Response<Body>,
    serial_number: String,
    container_id: String,
) -> Pin<Box<dyn Stream<Item = Result<Vec<u8>, ()>> + Send>> {
    Box::pin(
        ChunkedResponse::new(response.into_body().map(|s| s.map(|v| v.to_vec())))
            .map_err(|err| error!("Hyper error {:?}", err))
            .map(|messages| futures::stream::iter(messages.into_iter().flatten()))
            .flatten()
            .filter_map(move |message| {
                let res = serde_json::from_str(message.as_str())
                    .map_err(|err| {
                        // Hacky, but easiest way to remove loops
                        if !message.as_str().contains(container_id.as_str()) {
                            warn!("Error deserializing Journald message {:?}: {:?}", message, err)
                        }
                    })
                    .and_then(|json: JournaldJson| {
                        if container_id != json.container_id_full {
                            // Only log if message is NOT from ourselves!
                            debug!("Journald message: {:?}", json);

                            let message = convert_log_message(&json, serial_number.clone());

                            debug!("Publishing message: {:?}", message);

                            // Send message to pubsub even if it's from ourselves
                            serde_cbor::to_vec(&message)
                                .map_err(|err| warn!("Error serializing PublishLog message {:?}: {:?}", message, err))
                        } else {
                            Err(())
                        }
                    })
                    .ok();
                futures::future::ready(res)
            })
            .map(Ok),
    )
}

fn convert_log_message(json: &JournaldJson, serial_number: String) -> FullMonitoringMessage {
    lazy_static! {
        static ref CONTAINER_NAME_REGEX: Regex = Regex::new(r"_[0-9]*_[0-9]*$").unwrap();
    }

    let message = String::from_utf8_lossy(json.message.as_slice()).trim().to_string();
    let tag = CONTAINER_NAME_REGEX.replace(&json.container_name, "").to_string();

    let timestamp = json
        ._source_realtime_timestamp
        .parse()
        .map(|micros: i64| Utc.timestamp_nanos(micros * 1000))
        .unwrap_or_else(|_| Utc::now());

    let level = if message.to_lowercase().contains("error") {
        "ERROR".to_string()
    } else {
        "INFO".to_string()
    };

    let source_id = SourceId::from_str("actyxadacli").unwrap();

    FullMonitoringMessage::Log(vec![PublishLog::new(
        level,
        message,
        serial_number,
        tag,
        timestamp,
        source_id,
    )])
}
