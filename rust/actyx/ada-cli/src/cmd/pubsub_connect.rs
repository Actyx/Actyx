use crate::cmd;
use anyhow::Result;
use async_trait::async_trait;
use ax_config::StoreConfig;
use clap::{App, ArgMatches, SubCommand};
use store_core::BanyanStore;

pub struct Cmd;

pub fn args() -> App<'static, 'static> {
    SubCommand::with_name("pubsubConnect")
        .about("Uses a discovery pubsub topic to stay connected to as many peers as possible")
}

#[async_trait]
impl cmd::Command for Cmd {
    fn name(&self) -> &str {
        "pubsubConnect"
    }

    async fn run(&self, _matches: &ArgMatches<'_>, _config: StoreConfig, _store: BanyanStore) -> Result<()> {
        println!("Connecting to all the peers ..");
        println!("Note: There won't be any additional output from this tool.\nYou can however run it with `-vv` to see what's happening.");
        Ok(())
    }
}
