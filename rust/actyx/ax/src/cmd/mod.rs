use formats::{ActyxCliResult, Result};
use futures::{future, Future, Stream, StreamExt};
use serde::Serialize;
use std::{convert::TryInto, fmt, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult};

use crate::{node_connection::NodeConnection, private_key::AxPrivateKey};

pub mod apps;
pub mod events;
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
    /// the IP address or <host>:<admin port> of the node to perform the operation on.
    authority: NodeConnection,
    #[structopt(short, long)]
    /// File from which the identity (private key) for authentication is read.
    identity: Option<KeyPathWrapper>,
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

impl TryInto<AxPrivateKey> for Option<KeyPathWrapper> {
    type Error = ActyxOSError;
    fn try_into(self) -> Result<AxPrivateKey> {
        if let Some(path) = self {
            AxPrivateKey::from_file(path.0)
        } else {
            let private_key_path = AxPrivateKey::default_user_identity_path()?;
            AxPrivateKey::from_file(&private_key_path).map_err(move |e| {
                if e.code() == ActyxOSCode::ERR_PATH_INVALID {
                    ActyxOSError::new(
                        ActyxOSCode::ERR_USER_UNAUTHENTICATED,
                        format!(
                            "Unable to authenticate with node since no user keys found in \"{}\". \
                             To create user keys, run ax users keygen.",
                            private_key_path.display()
                        ),
                    )
                } else {
                    e
                }
            })
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

pub(crate) mod consts {
    use prettytable::format::{FormatBuilder, LinePosition, LineSeparator, TableFormat};
    lazy_static::lazy_static! {
        pub static ref TABLE_FORMAT: TableFormat = FormatBuilder::new()
            .column_separator('│')
            .borders('│')
            .separators(&[LinePosition::Top], LineSeparator::new('─', '┬', '┌', '┐'))
            .separators(&[LinePosition::Title], LineSeparator::new('─', '┼', '├', '┤'))
            .separators(&[LinePosition::Bottom], LineSeparator::new('─', '┴', '└', '┘'))
            .padding(1, 1)
            .build();
    }
}

pub trait AxCliCommand {
    type Opt: StructOpt;
    type Output: Serialize + 'static;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin>;
    fn pretty(result: Self::Output) -> String;
    fn output(opts: Self::Opt, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
        Box::new(Self::run(opts).for_each(move |item| {
            let exit = if item.is_ok() { 0 } else { 1 };
            if json {
                println!(
                    "{}",
                    serde_json::to_string(&ActyxCliResult::<Self::Output>::from(item)).unwrap()
                );
            } else {
                match item {
                    Ok(r) => println!("{}", Self::pretty(r)),
                    Err(err) => eprintln!("{}", err),
                }
            }
            if exit == 1 {
                std::process::exit(1)
            }
            future::ready(())
        }))
    }
}
