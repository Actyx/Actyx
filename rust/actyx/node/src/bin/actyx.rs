use anyhow::Result;
use anyhow::{anyhow, Context};
use derive_more::{Display, Error};
use node::{init_shutdown_ceremony, shutdown_ceremony, ApplicationState, BindTo, BindToOpts, Runtime};
use std::str::FromStr;
use std::{convert::TryInto, path::PathBuf};
use structopt::StructOpt;
use util::version::NodeVersion;

#[derive(Debug)]
enum Color {
    Off,
    Auto,
    On,
}

impl FromStr for Color {
    type Err = NoColor;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "1" | "on" | "true" => Ok(Self::On),
            "0" | "off" | "false" => Ok(Self::Off),
            "auto" => Ok(Self::Auto),
            _ => Err(NoColor),
        }
    }
}

#[derive(Debug, Display, Error)]
#[display(fmt = "allowed values are 1, on, true, 0, off, false, auto (case insensitive)")]
struct NoColor;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "actyx",
    about = concat!("\n", include_str!("../../../../../NOTICE")),
    help_message = "Print help information (use --help for more details)",
    after_help = "For one-off log verbosity override, you may start with the environment variable \
        RUST_LOG set to “debug” or “node=debug,info” (the former logs all debug messages while \
        the latter logs at debug level for the “node” code module and info level for everything \
        else).
        ",
    rename_all = "kebab-case"
)]
struct Opts {
    #[structopt(
        long,
        env = "ACTYX_PATH",
        long_help = "Path where to store all the data of the Actyx node. \
            Defaults to creating <current working dir>/actyx-data"
    )]
    /// Path where to store all the data of the Actyx node.
    working_dir: Option<PathBuf>,

    #[structopt(flatten)]
    bind_options: BindToOpts,

    #[structopt(short, long, hidden = true)]
    random: bool,

    #[structopt(long)]
    /// This does not do anything; kept for backward-compatibility
    background: bool,

    #[structopt(long)]
    version: bool,

    #[structopt(
        long,
        env = "ACTYX_COLOR",
        long_help = "Control whether to use ANSI color sequences in log output. \
            Valid values (case insensitive) are 1, true, on, 0, false, off, auto \
            (default is on, auto only uses colour when stderr is a terminal). \
            Defaults to 1."
    )]
    /// Control whether to use ANSI color sequences in log output.
    log_color: Option<Color>,

    #[structopt(
        long,
        env = "ACTYX_LOG_JSON",
        long_help = "Output logs as JSON objects (one per line) if the value is \
            1, true, on or if stderr is not a terminal and the value is auto \
            (all case insensitive). Defaults to 0."
    )]
    /// Output logs as JSON objects (one per line)
    log_json: Option<Color>,
}

pub fn main() -> Result<()> {
    let Opts {
        working_dir,
        bind_options,
        random,
        version,
        background,
        log_color,
        log_json,
    } = Opts::from_args();

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

    if background {
        eprintln!("Notice: the `--background` flag is no longer used and will just be ignored.")
    }

    if version {
        println!("Actyx {}", NodeVersion::get());
    } else {
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

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let runtime: Runtime = Runtime::Linux;
        #[cfg(target_os = "windows")]
        let runtime: Runtime = Runtime::Windows;
        #[cfg(target_os = "android")]
        let runtime: Runtime = Runtime::Android;
        let app_handle = ApplicationState::spawn(working_dir, runtime, bind_to, log_no_color, log_as_json)?;

        shutdown_ceremony(app_handle);
    }

    Ok(())
}
