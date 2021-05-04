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
use util::version::NodeVersion;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "Actyx CLI",
    about = "The Actyx Command Line Interface (CLI) is a unified tool to manage your Actyx nodes"
)]
struct Opt {
    // unless("version") gives "methods in attributes are not allowed for subcommand"
    #[structopt(subcommand)]
    commands: Option<CommandsOpt>,
    /// Format output as JSON
    #[structopt(long, short, global = true)]
    json: bool,
    #[structopt(flatten)]
    verbosity: Verbosity,
    #[structopt(long)]
    version: bool,
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

    // Since we can't tell StructOpt that the subcommand is optional when --version is specified, we have to do this
    // rigmarole
    match opt {
        Opt { version: true, .. } => {
            println!("Actyx CLI {}", NodeVersion::get());
        }
        Opt {
            commands: Some(cmd),
            json,
            ..
        } => {
            match cmd {
                CommandsOpt::Nodes(opts) => cmd::nodes::run(opts, json).await,
                CommandsOpt::Logs(opts) => cmd::logs::run(opts, json).await,
                CommandsOpt::Settings(opts) => cmd::settings::run(opts, json).await,
                CommandsOpt::Swarms(opts) => cmd::swarms::run(opts, json).await,
                CommandsOpt::Users(opts) => cmd::users::run(opts, json).await,
                CommandsOpt::Internal(opts) => cmd::internal::run(opts, json).await,
            };
        }
        _ => {
            let mut app = Opt::clap();
            app.write_long_help(&mut std::io::stderr()).unwrap();
        }
    }
}
