use anyhow::Result;
use anyhow::{anyhow, Context};
use node::{shutdown_ceremony, ApplicationState, BindTo, BindToOpts, Runtime};
use std::{convert::TryInto, path::PathBuf};
use structopt::StructOpt;
use util::version::NodeVersion;

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

    /// Disable colored logging output
    #[structopt(long)]
    log_no_color: bool,

    /// Output logs as JSON objects (one per line)
    #[structopt(long)]
    log_json: bool,
}

pub fn main() -> Result<()> {
    let Opts {
        working_dir,
        bind_options,
        version,
        background,
        log_no_color,
        log_json,
    } = Opts::from_args();

    eprintln!("log_no_color: {}", log_no_color);
    eprintln!("log_json: {}", log_json);

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
            if log_no_color { Some(false) } else { None },
            if log_json { Some(true) } else { None },
        )?;

        shutdown_ceremony(app_handle);
    }

    Ok(())
}
