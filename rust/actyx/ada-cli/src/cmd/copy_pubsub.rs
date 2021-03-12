use crate::cmd;
use anyhow::Result;
use async_trait::async_trait;
use ax_config::StoreConfig;
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::StreamExt;
use store_core::BanyanStore;

pub struct Cmd;

pub fn args() -> App<'static, 'static> {
    SubCommand::with_name("copyPubSub")
        .about("Copy selection of pubsub traffic from one topic to the other (converting them to JSON)")
        .arg(
            Arg::with_name("from")
                .help("Topic to listen on")
                .long("from")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("to")
                .help("Topic to publish to")
                .long("to")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("exclude")
                .help("do not forward messages containing this string in their JSON serialization")
                .long("exclude")
                .required(false)
                .takes_value(true),
        )
}

#[async_trait]
impl cmd::Command for Cmd {
    fn name(&self) -> &str {
        "copyPubSub"
    }

    async fn run(&self, matches: &ArgMatches<'_>, _config: StoreConfig, store: BanyanStore) -> Result<()> {
        let topic_from = String::from(matches.value_of("from").expect("Topic is mandatory"));
        let topic_to = String::from(matches.value_of("to").expect("Topic to send is mandatory"));
        let exclude = matches.value_of("exclude").map(|x| x.to_owned());
        let client = store.ipfs();
        let mut stream = client.subscribe(&topic_from).unwrap();
        while let Some(msg) = stream.next().await {
            let msg = match util::serde_util::from_json_or_cbor_slice::<serde_value::Value>(msg.as_slice()) {
                Ok(msg) => msg,
                Err(err) => {
                    eprintln!("Error reading from ipfs topic {}: {}", topic_from, err);
                    continue;
                }
            };
            let mut text = serde_json::to_string(&msg).unwrap();
            text.push('\n');
            let mut publish = true;
            if let Some(exclude) = &exclude {
                if text.contains(exclude) {
                    publish = false
                }
            }
            if publish {
                let _ = client.publish(&topic_to, text.into());
            } else {
                print!("dropping line {}", text);
            }
        }
        Ok(())
    }
}
