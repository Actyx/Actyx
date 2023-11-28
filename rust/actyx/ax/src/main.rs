mod cmd;

use crate::cmd::{
    apps::AppsOpts, events::EventsOpts, internal::InternalOpts, nodes::NodesOpts, settings::SettingsOpts,
    swarms::SwarmsOpts, topics::TopicsOpts, users::UsersOpts,
};
use anyhow::{anyhow, Context, Result};
use ax_core::{
    node::{
        self, init_shutdown_ceremony,
        run::{Color, RunOpts},
        shutdown_ceremony, ApplicationState, BindTo, Runtime,
    },
    util::version::NodeVersion,
};
use clap::{ArgAction, Parser};
use std::{future::Future, process::exit};

#[derive(clap::Parser, Clone, Debug)]
#[command(
    name = "ax",
    about = concat!(
        "\nThe ax CLI is a unified tool to manage your ax nodes.\n\n",
        include_str!("../NOTICE")),
    version = ax_core::util::version::VERSION.as_str(),
    disable_help_subcommand = true,
    action = ArgAction::Version
)]
struct Opt {
    #[command(subcommand)]
    command: CommandsOpt,
    /// Format output as JSON
    #[arg(long, short, global = true)]
    json: bool,
    /// Set verbosity
    #[arg(short, global = true, action = ArgAction::Count)]
    verbosity: u8,
}

#[derive(clap::Subcommand, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
enum CommandsOpt {
    // clap 3 use variant order to order displayed help subcommands
    Run(node::run::RunOpts),
    #[command(subcommand, arg_required_else_help(true))]
    Events(EventsOpts),
    #[command(subcommand, arg_required_else_help(true))]
    Nodes(NodesOpts),
    #[command(subcommand, arg_required_else_help(true))]
    Topics(TopicsOpts),
    #[command(subcommand, arg_required_else_help(true))]
    Swarms(SwarmsOpts),
    #[command(subcommand, arg_required_else_help(true))]
    Apps(AppsOpts),
    #[command(subcommand, arg_required_else_help(true))]
    Settings(SettingsOpts),
    #[command(subcommand, arg_required_else_help(true))]
    Users(UsersOpts),
    #[command(subcommand, arg_required_else_help(true), hide = !superpowers())]
    Internal(InternalOpts),
}

// NOTE: Reimplement superpower check for internal

// impl FromArgMatches for CliSub {
//     fn from_arg_matches(matches: &ArgMatches) -> Result<Self, Error> {
//         match matches.subcommand() {
//             Some(("add", args)) => Ok(Self::Add(AddArgs::from_arg_matches(args)?)),
//             Some(("remove", args)) => Ok(Self::Remove(RemoveArgs::from_arg_matches(args)?)),
//             Some((_, _)) => Err(Error::raw(
//                 ErrorKind::InvalidSubcommand,
//                 "Valid subcommands are `add` and `remove`",
//             )),
//             None => Err(Error::raw(
//                 ErrorKind::MissingSubcommand,
//                 "Valid subcommands are `add` and `remove`",
//             )),
//         }
//     }
//     fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), Error> {
//         match matches.subcommand() {
//             Some(("add", args)) => *self = Self::Add(AddArgs::from_arg_matches(args)?),
//             Some(("remove", args)) => *self = Self::Remove(RemoveArgs::from_arg_matches(args)?),
//             Some((_, _)) => {
//                 return Err(Error::raw(
//                     ErrorKind::InvalidSubcommand,
//                     "Valid subcommands are `add` and `remove`",
//                 ))
//             }
//             None => (),
//         };
//         Ok(())
//     }
// }

// impl Subcommand for CliSub {
//     fn augment_subcommands(cmd: Command) -> Command {
//         cmd.subcommand(AddArgs::augment_args(Command::new("add")))
//             .subcommand(RemoveArgs::augment_args(Command::new("remove")))
//             .subcommand_required(true)
//     }
//     fn augment_subcommands_for_update(cmd: Command) -> Command {
//         cmd.subcommand(AddArgs::augment_args(Command::new("add")))
//             .subcommand(RemoveArgs::augment_args(Command::new("remove")))
//             .subcommand_required(true)
//     }
//     fn has_subcommand(name: &str) -> bool {
//         matches!(name, "add" | "remove")
//     }
// }

fn superpowers() -> bool {
    let var = std::env::var("HERE_BE_DRAGONS").unwrap_or_default();
    var == "zøg" || var == "zoeg"
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let Opt {
        command,
        json,
        verbosity,
    } = match Opt::try_parse() {
        Ok(o) => o,
        Err(e) => match e.kind() {
            clap::error::ErrorKind::DisplayHelp => {
                println!("{}\n", e.to_string());
                exit(0)
            }
            clap::error::ErrorKind::DisplayVersion => {
                println!();
                exit(0)
            }
            _ => e.exit(),
        },
    };

    match command {
        CommandsOpt::Run(opts) => run(opts)?,
        CommandsOpt::Apps(opts) => with_logger(cmd::apps::run(opts, json), verbosity).await,
        CommandsOpt::Nodes(opts) => with_logger(cmd::nodes::run(opts, json), verbosity).await,
        CommandsOpt::Settings(opts) => with_logger(cmd::settings::run(opts, json), verbosity).await,
        CommandsOpt::Swarms(opts) => with_logger(cmd::swarms::run(opts, json), verbosity).await,
        CommandsOpt::Users(opts) => with_logger(cmd::users::run(opts, json), verbosity).await,
        CommandsOpt::Internal(opts) => with_logger(cmd::internal::run(opts, json), verbosity).await,
        CommandsOpt::Events(opts) => with_logger(cmd::events::run(opts, json), verbosity).await,
        CommandsOpt::Topics(opts) => with_logger(cmd::topics::run(opts, json), verbosity).await,
    }
    Ok(())
}

async fn with_logger<T>(fut: impl Future<Output = T>, verbosity: u8) -> T {
    ax_core::util::setup_logger_with_level(verbosity);
    fut.await
}

// This method does not belong here, it belongs in ax-core
// we need to extract this and it's friends
pub fn run(
    RunOpts {
        working_dir,
        bind_options,
        random,
        version,
        log_color,
        log_json,
    }: RunOpts,
) -> Result<()> {
    let is_no_tty = atty::isnt(atty::Stream::Stderr);
    let log_no_color = match log_color {
        Some(Color::On) => false,
        Some(Color::Off) => true,
        Some(Color::Auto) => is_no_tty,
        None => false,
    };
    let log_as_json = match log_json {
        Some(Color::On) => true,
        Some(Color::Off) => false,
        Some(Color::Auto) => is_no_tty,
        None => false,
    };

    if version {
        println!("ax {}", NodeVersion::get());
        return Ok(());
    }

    let bind_to = if random {
        BindTo::random()?
    } else {
        bind_options.try_into()?
    };
    let working_dir = working_dir.ok_or_else(|| anyhow!("empty")).or_else(|_| -> Result<_> {
        Ok(std::env::current_dir()
            .context("getting current working directory")?
            .join("actyx-data"))
    })?;

    std::fs::create_dir_all(working_dir.clone())
        .with_context(|| format!("creating working directory `{}`", working_dir.display()))?;
    // printed by hand since things can fail before logging is set up and we want the user to know this
    eprintln!("using data directory `{}`", working_dir.display());

    // must be done before starting the application
    init_shutdown_ceremony();

    if cfg!(target_os = "android") {
        panic!("Unsupported platform");
    } else {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let runtime = Runtime::Linux;
        #[cfg(target_os = "windows")]
        let runtime = Runtime::Windows;

        let app_handle = ApplicationState::spawn(working_dir, runtime, bind_to, log_no_color, log_as_json)?;

        shutdown_ceremony(app_handle)?;
    }

    Ok(())
}
