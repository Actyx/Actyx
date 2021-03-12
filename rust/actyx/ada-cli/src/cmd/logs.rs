use crate::cmd;
use actyxos_sdk::event::SourceId;
use anyhow::Result;
use async_trait::async_trait;
use ax_config::StoreConfig;
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::StreamExt;
use lake::live::{LiveEvents, Topic};
use std::collections::BTreeSet;
use std::io::Write;
use std::str::FromStr;
use store_core::BanyanStore;
use trees::FullMonitoringMessage;

pub struct Cmd;

pub fn args() -> App<'static, 'static> {
    SubCommand::with_name("logs")
        .about("Trace out all logs from the given sources. Use \"all\" to see all log messages")
        .arg(
            Arg::with_name("monitoring_Topic")
                .help("Topic, where the store publish the log messages.")
                .required(true)
                .long("monitoring_topic")
                .short("m")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("sources")
                .help("Comma separated list of sources to filter for. Use \"all\" to see all log messages")
                .required(true)
                .takes_value(true)
                .index(1),
        )
        .arg(
            Arg::with_name("no_timestamp")
                .help("Remove the timestamp on the output")
                .required(false)
                .long("noTimestamp")
                .short("t")
                .takes_value(false),
        )
}

#[async_trait]
impl cmd::Command for Cmd {
    fn name(&self) -> &str {
        "logs"
    }

    async fn run(&self, matches: &ArgMatches<'_>, _config: StoreConfig, store: BanyanStore) -> Result<()> {
        let monitoring_topic = Topic(
            matches
                .value_of("monitoring_topic")
                .expect("monitoring_topic is mandatory")
                .to_string(),
        );
        let sorted_sources: BTreeSet<SourceId> = matches
            .value_of("sources")
            .map(|s| {
                s.trim()
                    .split(',')
                    .map(str::trim)
                    .map(|src| SourceId::from_str(src).expect("Source was in wrong format"))
                    .collect()
            })
            .unwrap_or_default();
        let no_ts = matches.is_present("no_timestamp");

        run_cmd(store, monitoring_topic, sorted_sources, no_ts).await;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_cmd(store: BanyanStore, monitoring_topic: Topic, sorted_sources: BTreeSet<SourceId>, no_ts: bool) {
    let client = store.ipfs();
    let response = LiveEvents::new(&client)
        .listen_on::<FullMonitoringMessage>(&monitoring_topic)
        .unwrap()
        .filter_map(|msg| {
            if let FullMonitoringMessage::Log(logs) = msg {
                futures::future::ready(Some(logs))
            } else {
                futures::future::ready(None)
            }
        })
        .for_each(move |logs| {
            let l_sorted_sources = sorted_sources.clone();
            let all = SourceId::from_str("all").unwrap();
            let show_all = l_sorted_sources.contains(&all);
            logs.iter()
                .filter(|log| show_all || l_sorted_sources.contains(&log.source_id()))
                .for_each(|log| {
                    let std_out = std::io::stdout();
                    let mut output = std_out.lock();
                    if !no_ts {
                        output
                            .write_fmt(format_args!("{}", log.timestamp()))
                            .expect("failed to write");
                    }
                    if show_all || l_sorted_sources.len() > 1 {
                        output
                            .write_fmt(format_args!(" {}:", log.source_id()))
                            .expect("failed to write");
                    } else if !no_ts {
                        output.write_all(b":").expect("failed to write");
                    }
                    output
                        .write_fmt(format_args!("{} {} - {}\n", log.level(), log.tag(), log.message()))
                        .expect("failed to write");
                });
            futures::future::ready(())
        });
    response.await;
}
