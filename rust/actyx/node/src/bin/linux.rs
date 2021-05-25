use anyhow::Context;
use node::{shutdown_ceremony, ApplicationState, BindTo, BindToOpts, Runtime};
use std::{convert::TryInto, path::PathBuf};
use structopt::StructOpt;
use util::version::NodeVersion;

#[derive(StructOpt, Debug)]
#[structopt(name = "actyx", about = "Actyx on Linux", rename_all = "kebab-case")]
struct Opts {
    #[structopt(long, env = "ACTYX_PATH")]
    /// Path where to store all the data of the Actyx node
    /// defaults to creating <current working dir>/actyx-data
    working_dir: Option<PathBuf>,

    #[structopt(flatten)]
    bind_options: BindToOpts,

    #[structopt(long)]
    version: bool,
}

fn main() -> anyhow::Result<()> {
    let Opts {
        working_dir: maybe_working_dir,
        bind_options,
        version,
    } = Opts::from_args();

    if version {
        println!("actyx {}", NodeVersion::get());
    } else {
        let bind_to: BindTo = bind_options.try_into()?;
        let working_dir = maybe_working_dir.unwrap_or_else(|| std::env::current_dir().unwrap().join("actyx-data"));
        std::fs::create_dir_all(working_dir.clone())
            .with_context(|| format!("creating working directory `{}`", working_dir.display()))?;
        // printed by hand since things can fail before logging is set up and we want the user to know this
        eprintln!("INFO using data directory `{}`", working_dir.display());

        let app_handle = ApplicationState::spawn(working_dir, Runtime::Linux, bind_to)?;

        shutdown_ceremony(app_handle);
    }

    Ok(())
}
