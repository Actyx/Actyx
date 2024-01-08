#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://developer.actyx.com/img/logo.svg")]
#![doc(html_favicon_url = "https://developer.actyx.com/img/favicon.ico")]

mod cmd;

use crate::cmd::{
    apps::AppsOpts, determine_ax_default_data_dir, events::EventsOpts, internal::InternalOpts, nodes::NodesOpts,
    run::Color, settings::SettingsOpts, swarms::SwarmsOpts, topics::TopicsOpts, users::UsersOpts,
};
use anyhow::{Context, Result};
use ax_core::{
    node::{init_shutdown_ceremony, shutdown_ceremony, ApplicationState, BindTo, Runtime},
    util::version::NodeVersion,
    NODE_VERSION,
};
use clap::{ArgAction, Args, Parser};
use clap_complete::Shell;
use cmd::run::RunOpts;
use std::{
    env::consts::{ARCH, OS},
    future::Future,
    process::exit,
};

#[derive(clap::Parser, Clone, Debug)]
#[command(
    name = "ax",
    about = concat!(
        "\nThe ax CLI is a unified tool to manage your ax nodes.\n\n",
        include_str!("../NOTICE")),
    version = ax_core::util::version::VERSION.as_str(),
    propagate_version = true,
    disable_help_subcommand = true
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
    Run(RunOpts),
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
    /// Generate completion scripts for your shell
    ///
    /// For example, on bash use `eval "$(ax complete bash)"` to setup completion.
    /// On zsh it works analogously, but you need to ensure that `compinit` has been run before.
    Complete {
        shell: Shell,
    },
}

fn superpowers() -> bool {
    let var = std::env::var("HERE_BE_DRAGONS").unwrap_or_default();
    var == "zøg" || var == "zoeg"
}

#[cfg(debug_assertions)]
const PROFILE: &str = "debug";
#[cfg(not(debug_assertions))]
const PROFILE: &str = "release";

const GIT_HASH: &str = match option_env!("GITHUB_SHA") {
    Some(hash) => hash,
    // This is for cargo installations and builds
    None => "cargo",
};

fn main() -> Result<()> {
    let version = NodeVersion {
        profile: PROFILE.to_string(),
        target: format!("{}-{}", OS, ARCH),
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_hash: GIT_HASH.to_string(),
    };
    NODE_VERSION.set(version).expect("V should be unset at this point");

    let Opt {
        command,
        json,
        verbosity,
    } = Opt::parse();

    match command {
        CommandsOpt::Run(opts) => run(opts)?,
        CommandsOpt::Apps(opts) => handle_cmd(cmd::apps::run(opts, json), verbosity),
        CommandsOpt::Nodes(opts) => handle_cmd(cmd::nodes::run(opts, json), verbosity),
        CommandsOpt::Settings(opts) => handle_cmd(cmd::settings::run(opts, json), verbosity),
        CommandsOpt::Swarms(opts) => handle_cmd(cmd::swarms::run(opts, json), verbosity),
        CommandsOpt::Users(opts) => handle_cmd(cmd::users::run(opts, json), verbosity),
        CommandsOpt::Internal(opts) => handle_cmd(cmd::internal::run(opts, json), verbosity),
        CommandsOpt::Events(opts) => handle_cmd(cmd::events::run(opts, json), verbosity),
        CommandsOpt::Topics(opts) => handle_cmd(cmd::topics::run(opts, json), verbosity),
        CommandsOpt::Complete { shell } => {
            let mut cmd = Opt::augment_args(clap::Command::new("ax"));
            clap_complete::generate(shell, &mut cmd, "ax", &mut std::io::stdout());
            exit(0);
        }
    }
    Ok(())
}

fn handle_cmd(fut: impl Future<Output = ()>, verbosity: u8) {
    ax_core::util::setup_logger_with_level(verbosity);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build runtime");
    rt.block_on(fut);
}

// This method does not belong here, it belongs in ax-core
// we need to extract this and it's friends
pub fn run(
    RunOpts {
        working_dir,
        bind_options,
        random,
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

    let bind_to = if random {
        BindTo::random()?
    } else {
        bind_options.try_into()?
    };

    let working_dir = if let Some(working_dir) = working_dir {
        working_dir
    } else {
        determine_ax_default_data_dir()?
    };

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
