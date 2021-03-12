#![macro_use]
extern crate prettytable;

mod cmd;
mod node_connection;
mod private_key;

use cmd::{
    internal::InternalOpts, logs::LogsOpts, nodes::NodesOpts, settings::SettingsOpts, swarms::SwarmsOpts,
    users::UsersOpts, Verbosity,
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "Actyx CLI",
    about = "The Actyx Command Line Interface (CLI) is a unified tool to manage your Actyx nodes"
)]
struct Opt {
    #[structopt(subcommand)]
    commands: CommandsOpt,
    /// Format output as JSON
    #[structopt(long, short, global = true)]
    json: bool,
    #[structopt(flatten)]
    verbosity: Verbosity,
}

#[derive(StructOpt, Debug)]
#[allow(clippy::clippy::large_enum_variant)]
enum CommandsOpt {
    // structopt will use the enum variant name in lowercase as a subcommand
    Settings(SettingsOpts),
    Swarms(SwarmsOpts),
    Logs(LogsOpts),
    Nodes(NodesOpts),
    Users(UsersOpts),
    #[structopt(setting(structopt::clap::AppSettings::Hidden), name = "_internal")]
    Internal(InternalOpts),
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let opt = Opt::from_args();
    match opt.commands {
        CommandsOpt::Nodes(opts) => cmd::nodes::run(opts, opt.json).await,
        CommandsOpt::Logs(opts) => cmd::logs::run(opts, opt.json).await,
        CommandsOpt::Settings(opts) => cmd::settings::run(opts, opt.json).await,
        CommandsOpt::Swarms(opts) => cmd::swarms::run(opts, opt.json).await,
        CommandsOpt::Users(opts) => cmd::users::run(opts, opt.json).await,
        CommandsOpt::Internal(opts) => cmd::internal::run(opts, opt.json).await,
    };
}
