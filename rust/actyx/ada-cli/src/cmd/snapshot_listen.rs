use crate::cmd;
use anyhow::Result;
use async_trait::async_trait;
use ax_config::StoreConfig;
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::StreamExt;
use store_core::live::{LiveEvents, Topic};
use store_core::BanyanStore;
use trees::PublishSnapshot;

pub struct Cmd;

pub fn args() -> App<'static, 'static> {
    SubCommand::with_name("listenSnapshot")
        .about("Listen to snapshots on a given topic")
        .arg(
            Arg::with_name("TOPIC")
                .help("Topic to listen on")
                .required(true)
                .index(1),
        )
}

#[async_trait]
impl cmd::Command for Cmd {
    fn name(&self) -> &str {
        "listenSnapshot"
    }

    async fn run(&self, matches: &ArgMatches<'_>, _config: StoreConfig, store: BanyanStore) -> Result<()> {
        let topic = matches.value_of("TOPIC").expect("Topic is mandatory");
        let client = store.ipfs();
        let live = LiveEvents::new(client);

        let completion = live
            .listen_on::<PublishSnapshot>(&Topic(topic.to_string()))
            .unwrap()
            .for_each(|snapshot| {
                println!("{}", snapshot);
                futures::future::ready(())
            });

        completion.await;
        Ok(())
    }
}
