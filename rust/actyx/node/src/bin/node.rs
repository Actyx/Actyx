use anyhow::Result;
use anyhow::{anyhow, Context};
use derive_more::{Display, Error};
use node::{shutdown_ceremony, ApplicationState, BindTo, BindToOpts, Runtime};
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
        rename_all = "kebab-case"
    )]
struct Opts {
    #[structopt(long, env = "ACTYX_PATH")]
    /// Path where to store all the data of the Actyx node
    /// defaults to creating <current working dir>/actyx-data
    working_dir: Option<PathBuf>,

    #[structopt(flatten)]
    bind_options: BindToOpts,

    #[structopt(long)]
    /// This does not do anything; kept for backward-compatibility
    background: bool,

    #[structopt(long)]
    version: bool,

    /// Control whether to use ANSI color sequences in log output.
    /// Valid values (case insensitive) are 1, true, on, 0, false, off, auto
    /// (default is on, auto only uses colour when stderr is a terminal).
    #[structopt(long, env = "ACTYX_COLOR")]
    log_color: Option<Color>,

    /// Output logs as JSON objects (one per line)
    #[structopt(long, env = "ACTYX_LOG_JSON")]
    log_json: bool,
}

pub fn main() -> Result<()> {
    let Opts {
        working_dir,
        bind_options,
        version,
        background,
        log_color,
        log_json,
    } = Opts::from_args();

    eprintln!("log_color: {:?}", log_color);
    eprintln!("log_json: {}", log_json);

    let log_no_color = match log_color {
        Some(Color::On) => false,
        Some(Color::Off) => true,
        Some(Color::Auto) => atty::isnt(atty::Stream::Stderr),
        None => false,
    };

    if background {
        eprintln!("Notice: the `--background` flag is no longer used and will just be ignored.")
    }

    if version {
        println!("Actyx {}", NodeVersion::get());
    } else {
        let bind_to: BindTo = bind_options.try_into()?;
        let working_dir = working_dir.ok_or_else(|| anyhow!("empty")).or_else(|_| -> Result<_> {
            Ok(std::env::current_dir()
                .context("getting current working directory")?
                .join("actyx-data"))
        })?;

        std::fs::create_dir_all(working_dir.clone())
            .with_context(|| format!("creating working directory `{}`", working_dir.display()))?;
        // printed by hand since things can fail before logging is set up and we want the user to know this
        eprintln!("using data directory `{}`", working_dir.display());

        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let runtime: Runtime = Runtime::Linux;
        #[cfg(target_os = "windows")]
        let runtime: Runtime = Runtime::Windows;
        #[cfg(target_os = "android")]
        let runtime: Runtime = Runtime::Android;
        let app_handle = ApplicationState::spawn(
            working_dir,
            runtime,
            bind_to,
            log_no_color,
            if log_json { Some(true) } else { None },
        )?;

        shutdown_ceremony(app_handle);
    }

    Ok(())
}
