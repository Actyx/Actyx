mod cmd;
mod node_connection;
mod private_key;

use cmd::{
    apps::AppsOpts, internal::InternalOpts, nodes::NodesOpts, settings::SettingsOpts, swarms::SwarmsOpts,
    users::UsersOpts, Verbosity,
};
use structopt::{
    clap::{App, AppSettings::SubcommandRequiredElseHelp, ArgMatches, SubCommand},
    StructOpt, StructOptInternal,
};
use util::version::NodeVersion;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "Actyx CLI",
    about = "The Actyx Command Line Interface (CLI) is a unified tool to manage your Actyx nodes",
    no_version
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

#[derive(Debug)]
#[allow(clippy::clippy::large_enum_variant)]
enum CommandsOpt {
    // structopt will use the enum variant name in lowercase as a subcommand
    Apps(AppsOpts),
    Settings(SettingsOpts),
    Swarms(SwarmsOpts),
    Nodes(NodesOpts),
    Users(UsersOpts),
    Internal(InternalOpts),
}

impl StructOpt for CommandsOpt {
    fn clap<'a, 'b>() -> App<'a, 'b> {
        let app = App::new("ax").setting(SubcommandRequiredElseHelp);
        Self::augment_clap(app)
    }

    fn from_clap(matches: &ArgMatches<'_>) -> Self {
        Self::from_subcommand(matches.subcommand()).expect("wat")
    }
}

impl StructOptInternal for CommandsOpt {
    fn augment_clap<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        let app = app.subcommands(vec![
            AppsOpts::augment_clap(SubCommand::with_name("apps")).setting(SubcommandRequiredElseHelp),
            SettingsOpts::augment_clap(SubCommand::with_name("settings")).setting(SubcommandRequiredElseHelp),
            SwarmsOpts::augment_clap(SubCommand::with_name("swarms")).setting(SubcommandRequiredElseHelp),
            NodesOpts::augment_clap(SubCommand::with_name("nodes")).setting(SubcommandRequiredElseHelp),
            UsersOpts::augment_clap(SubCommand::with_name("users")).setting(SubcommandRequiredElseHelp),
        ]);
        if superpowers() {
            app.subcommand(
                InternalOpts::augment_clap(SubCommand::with_name("internal")).setting(SubcommandRequiredElseHelp),
            )
        } else {
            app
        }
    }

    fn from_subcommand<'a, 'b>(sub: (&'b str, Option<&'b ArgMatches<'a>>)) -> Option<Self>
    where
        Self: std::marker::Sized,
    {
        match sub {
            ("apps", Some(matches)) => Some(CommandsOpt::Apps(AppsOpts::from_clap(matches))),
            ("settings", Some(matches)) => Some(CommandsOpt::Settings(SettingsOpts::from_clap(matches))),
            ("swarms", Some(matches)) => Some(CommandsOpt::Swarms(SwarmsOpts::from_clap(matches))),
            ("nodes", Some(matches)) => Some(CommandsOpt::Nodes(NodesOpts::from_clap(matches))),
            ("users", Some(matches)) => Some(CommandsOpt::Users(UsersOpts::from_clap(matches))),
            ("internal", Some(matches)) if superpowers() => {
                Some(CommandsOpt::Internal(InternalOpts::from_clap(matches)))
            }
            _ => None,
        }
    }
}

fn superpowers() -> bool {
    let var = std::env::var("HERE_BE_DRAGONS").unwrap_or_default();
    var == "zøg" || var == "zoeg"
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let opt = Opt::from_args();

    // Since we can't tell StructOpt that the subcommand is optional when --version is specified, we have to do this
    // rigmarole
    match opt {
        Opt { version: true, .. } => {
            println!("Actyx CLI {}", NodeVersion::get_cli());
        }
        Opt {
            commands: Some(cmd),
            json,
            ..
        } => {
            match cmd {
                CommandsOpt::Apps(opts) => cmd::apps::run(opts, json).await,
                CommandsOpt::Nodes(opts) => cmd::nodes::run(opts, json).await,
                CommandsOpt::Settings(opts) => cmd::settings::run(opts, json).await,
                CommandsOpt::Swarms(opts) => cmd::swarms::run(opts, json).await,
                CommandsOpt::Users(opts) => cmd::users::run(opts, json).await,
                CommandsOpt::Internal(opts) => cmd::internal::run(opts, json).await,
            };
        }
        _ => {
            let mut app = Opt::clap();
            app.write_long_help(&mut std::io::stderr()).unwrap();
            eprintln!();
        }
    }
}
