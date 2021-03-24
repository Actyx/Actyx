use anyhow::Result;
use async_trait::async_trait;
use ax_config::StoreConfig;
use clap::ArgMatches;
use swarm::BanyanStore;

pub mod copy_pubsub;
pub mod monitor_pubsub;
pub mod pubsub_connect;
pub mod pubsub_to_pg;

/// All subcommands implement this trait and then register themselves in `main`. When the
/// app starts up, the matching subcommand is found ([name](Command::name) is used for this) then
/// the [run](Command::run) method is called with the global options, and the `ArgMatches` relevant
/// for this subcommand.
#[async_trait]
pub trait Command {
    /// Name of the subcommand. Used by `main` to find the right subcommand from command line
    /// arguments and launch the [run](run) method.
    fn name(&self) -> &str;

    /// If the command line arguments match this subcommand, then this method will be called.
    /// This is where the actual task of the subcommand is performed.
    async fn run(&self, matches: &ArgMatches<'_>, config: StoreConfig, store: BanyanStore) -> Result<()>;
}
