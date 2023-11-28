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
use std::{future::Future, process::exit};
use structopt::{
    clap::{App, AppSettings, ArgMatches, ErrorKind, SubCommand},
    StructOpt, StructOptInternal,
};

#[derive(StructOpt, Debug)]
#[structopt(
    name = "ax",
    about = concat!(
        "\nThe ax CLI is a unified tool to manage your ax nodes.\n\n",
        include_str!("../NOTICE")),
    version = ax_core::util::version::VERSION.as_str(),
)]
struct Opt {
    #[structopt(subcommand)]
    command: CommandsOpt,
    /// Format output as JSON
    #[structopt(long, short, global = true)]
    json: bool,
    #[structopt(short, parse(from_occurrences), global = true)]
    verbosity: u8,
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
    Topics(TopicsOpts),
    Run(node::run::RunOpts),
}

impl StructOpt for CommandsOpt {
    fn clap<'a, 'b>() -> App<'a, 'b> {
        let app = App::new("ax").setting(AppSettings::SubcommandRequiredElseHelp);
        Self::augment_clap(app)
    }

    fn from_clap(matches: &ArgMatches<'_>) -> Self {
        Self::from_subcommand(matches.subcommand()).expect("wat")
    }
}

impl StructOptInternal for CommandsOpt {
    fn augment_clap<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        let app = app.subcommands(vec![
            node::run::RunOpts::augment_clap(SubCommand::with_name("run")),
            AppsOpts::augment_clap(SubCommand::with_name("apps")).setting(AppSettings::SubcommandRequiredElseHelp),
            SettingsOpts::augment_clap(SubCommand::with_name("settings"))
                .setting(AppSettings::SubcommandRequiredElseHelp),
            SwarmsOpts::augment_clap(SubCommand::with_name("swarms")).setting(AppSettings::SubcommandRequiredElseHelp),
            NodesOpts::augment_clap(SubCommand::with_name("nodes")).setting(AppSettings::SubcommandRequiredElseHelp),
            UsersOpts::augment_clap(SubCommand::with_name("users")).setting(AppSettings::SubcommandRequiredElseHelp),
            EventsOpts::augment_clap(SubCommand::with_name("events")).setting(AppSettings::SubcommandRequiredElseHelp),
            TopicsOpts::augment_clap(SubCommand::with_name("topics").setting(AppSettings::SubcommandRequiredElseHelp)),
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

    fn from_subcommand<'a>(sub: (&'a str, Option<&'a ArgMatches<'_>>)) -> Option<Self>
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
            ("topics", Some(matches)) => Some(CommandsOpt::Topics(TopicsOpts::from_clap(matches))),
            ("run", Some(matches)) => Some(CommandsOpt::Run(node::run::RunOpts::from_clap(matches))),
            _ => None,
        }
    }
}

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
