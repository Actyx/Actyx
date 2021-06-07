use formats::{ActyxCliResult, Result};
use futures::{future, Future, Stream, StreamExt};
use serde::Serialize;
use std::{convert::TryInto, fmt, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult};

use crate::{node_connection::NodeConnection, private_key::AxPrivateKey};

pub mod apps;
mod formats;
pub(crate) mod internal;
pub mod nodes;
pub mod settings;
pub mod swarms;
pub mod users;

#[derive(StructOpt, Debug)]
pub struct Verbosity {
    /// Verbosity level. Add more v for higher verbosity (-v, -vv, -vvv, etc.).
    #[structopt(short, parse(from_occurrences = util::set_log_level), global = true)]
    verbosity: u64,
}

#[derive(StructOpt, Debug)]
pub struct ConsoleOpt {
    #[structopt(name = "NODE", required = true)]
    /// Node ID or, if using `--local`, the IP address of the node to perform
    /// the operation on.
    authority: NodeConnection,
    #[structopt(short, long)]
    /// Process over local network
    local: bool,
    #[structopt(short, long, default_value)]
    /// File from which the identity (private key) for authentication is read.
    identity: KeyPathWrapper,
}

#[derive(Debug)]
/// Newtype wrapper around a path to key material, to be used with
/// structopt/clap.
pub struct KeyPathWrapper(PathBuf);

impl FromStr for KeyPathWrapper {
    type Err = ActyxOSError;
    fn from_str(s: &str) -> Result<Self> {
        Ok(Self(s.into()))
    }
}

impl fmt::Display for KeyPathWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

impl Default for KeyPathWrapper {
    fn default() -> Self {
        let private_key_path = AxPrivateKey::default_user_identity_path().unwrap_or_else(|e| {
            eprintln!("Error getting config dir: {}", e);
            ".".into()
        });
        Self(private_key_path)
    }
}

impl TryInto<AxPrivateKey> for KeyPathWrapper {
    type Error = ActyxOSError;
    fn try_into(self) -> Result<AxPrivateKey> {
        AxPrivateKey::from_file(self.0)
    }
}

impl ConsoleOpt {
    fn assert_local(&self) -> Result<()> {
        if !self.local {
            Err(ActyxOSCode::ERR_INVALID_INPUT
                .with_message("This version of ax currently only supports local interactions using the --local flag."))
        } else {
            Ok(())
        }
    }
}

/// Returns the data directory for Actyx. Does not create the folders!
/// https://docs.rs/dirs/3.0.1/dirs/fn.config_dir.html
///
/// Platform    Value                               Example
/// Linux       $XDG_CONFIG_HOME or $HOME/.config   /home/alice/.config
/// macOS       $HOME/Library/Application Support   /Users/Alice/Library/Application Support
/// Windows     {FOLDERID_RoamingAppData}           C:\Users\Alice\AppData\Roaming
pub(crate) fn get_data_dir() -> ActyxOSResult<PathBuf> {
    let data_dir = dirs::config_dir().ok_or_else(|| ActyxOSError::internal("Can't get user's config dir"))?;

    Ok(data_dir.join("actyx"))
}

pub trait AxCliCommand {
    type Opt: StructOpt;
    type Output: Serialize + 'static;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin>;
    fn pretty(result: Self::Output) -> String;
    fn output(opts: Self::Opt, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
        Box::new(Self::run(opts).for_each(move |item| {
            let exit = if item.is_ok() { 0 } else { 1 };
            println!(
                "{}",
                if json {
                    serde_json::to_string::<ActyxCliResult<Self::Output>>(&item.into()).unwrap()
                } else {
                    match item {
                        Ok(r) => Self::pretty(r),
                        Err(err) => format!("{}", err),
                    }
                }
            );
            if exit == 1 {
                std::process::exit(1)
            }
            future::ready(())
        }))
    }
}
