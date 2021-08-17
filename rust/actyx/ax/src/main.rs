mod cmd;
mod node_connection;
mod private_key;

use cmd::{
    apps::AppsOpts, events::EventsOpts, internal::InternalOpts, nodes::NodesOpts, settings::SettingsOpts,
    swarms::SwarmsOpts, users::UsersOpts, Verbosity,
};
use std::process::exit;
use structopt::{
    clap::{App, AppSettings, ArgMatches, ErrorKind, SubCommand},
    StructOpt, StructOptInternal,
};

#[derive(StructOpt, Debug)]
#[structopt(
    name = "Actyx CLI",
    about = concat!(
        "\nThe Actyx Command Line Interface (CLI) is a unified tool to manage your Actyx nodes.\n\n",
        include_str!("../../../../NOTICE")),
    version = env!("AX_CLI_VERSION"),
)]
struct Opt {
    #[structopt(subcommand)]
    command: CommandsOpt,
    /// Format output as JSON
    #[structopt(long, short, global = true)]
    json: bool,
    #[structopt(flatten)]
    verbosity: Verbosity,
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum CommandsOpt {
    // structopt will use the enum variant name in lowercase as a subcommand
    Apps(AppsOpts),
    Settings(SettingsOpts),
    Swarms(SwarmsOpts),
    Nodes(NodesOpts),
    Users(UsersOpts),
    Internal(InternalOpts),
    Events(EventsOpts),
}

impl StructOpt for CommandsOpt {
    fn clap<'a, 'b>() -> App<'a, 'b> {
        let app = App::new("Actyx CLI").setting(AppSettings::SubcommandRequiredElseHelp);
        Self::augment_clap(app)
    }

    fn from_clap(matches: &ArgMatches<'_>) -> Self {
        Self::from_subcommand(matches.subcommand()).expect("wat")
    }
}

impl StructOptInternal for CommandsOpt {
    fn augment_clap<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        let app = app.subcommands(vec![
            AppsOpts::augment_clap(SubCommand::with_name("apps")).setting(AppSettings::SubcommandRequiredElseHelp),
            SettingsOpts::augment_clap(SubCommand::with_name("settings"))
                .setting(AppSettings::SubcommandRequiredElseHelp),
            SwarmsOpts::augment_clap(SubCommand::with_name("swarms")).setting(AppSettings::SubcommandRequiredElseHelp),
            NodesOpts::augment_clap(SubCommand::with_name("nodes")).setting(AppSettings::SubcommandRequiredElseHelp),
            UsersOpts::augment_clap(SubCommand::with_name("users")).setting(AppSettings::SubcommandRequiredElseHelp),
            EventsOpts::augment_clap(SubCommand::with_name("events")).setting(AppSettings::SubcommandRequiredElseHelp),
        ]);
        if superpowers() {
            app.subcommand(
                InternalOpts::augment_clap(SubCommand::with_name("internal"))
                    .setting(AppSettings::SubcommandRequiredElseHelp),
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
            ("events", Some(matches)) => Some(CommandsOpt::Events(EventsOpts::from_clap(matches))),
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
    let Opt {
        command,
        json,
        verbosity: _verbosity,
    } = match Opt::from_args_safe() {
        Ok(o) => o,
        Err(e) => match e.kind {
            ErrorKind::HelpDisplayed => {
                println!("{}\n", e.message);
                exit(0)
            }
            ErrorKind::VersionDisplayed => {
                println!();
                exit(0)
            }
            _ => e.exit(),
        },
    };

    util::setup_logger();

    match command {
        CommandsOpt::Apps(opts) => cmd::apps::run(opts, json).await,
        CommandsOpt::Nodes(opts) => cmd::nodes::run(opts, json).await,
        CommandsOpt::Settings(opts) => cmd::settings::run(opts, json).await,
        CommandsOpt::Swarms(opts) => cmd::swarms::run(opts, json).await,
        CommandsOpt::Users(opts) => cmd::users::run(opts, json).await,
        CommandsOpt::Internal(opts) => cmd::internal::run(opts, json).await,
        CommandsOpt::Events(opts) => cmd::events::run(opts, json).await,
    }
}
