use anyhow::Result;
#[cfg(not(windows))]
mod linux {
    use super::*;
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
        version: bool,

        #[structopt(long)]
        no_color: bool,
    }

    pub fn main() -> Result<()> {
        let Opts {
            working_dir,
            bind_options,
            version,
            no_color,
        } = Opts::from_args();

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
            eprintln!("no-color: {}", no_color);

            let app_handle = ApplicationState::spawn(
                working_dir,
                Runtime::Linux,
                bind_to,
                if no_color { Some(false) } else { None },
            )?;

            shutdown_ceremony(app_handle);
        }

        Ok(())
    }
}
#[cfg(windows)]
mod windows {
    use super::*;
    pub fn main() -> Result<()> {
        panic!("This program is not intended to run on Windows. Maybe you were looking for \"actyx\"?");
    }
}
fn main() -> Result<()> {
    #[cfg(not(windows))]
    return linux::main();
    #[cfg(windows)]
    return windows::main();
}
