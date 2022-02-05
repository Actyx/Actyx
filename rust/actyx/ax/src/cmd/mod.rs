use formats::{ActyxCliResult, Result};
use futures::{channel::mpsc::Sender, future, Future, Stream, StreamExt};
use serde::Serialize;
use std::{convert::TryFrom, fmt, net::ToSocketAddrs, path::PathBuf, str::FromStr};
use structopt::StructOpt;
use util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult};

use crate::{
    node_connection::{connect, mk_swarm, Task},
    private_key::AxPrivateKey,
};
use libp2p::{multiaddr::Protocol, Multiaddr, PeerId};

pub mod apps;
pub mod events;
mod formats;
pub(crate) mod internal;
pub mod nodes;
pub mod settings;
pub mod swarms;
pub mod users;

#[derive(Debug, Clone)]
pub struct Authority {
    pub original: String,
    pub addrs: Vec<Multiaddr>,
}

impl FromStr for Authority {
    type Err = ActyxOSError;

    fn from_str(s: &str) -> Result<Self> {
        let original = s.to_owned();
        if let Ok(m) = s.parse::<Multiaddr>() {
            Ok(Self {
                original,
                addrs: vec![m],
            })
        } else if let Ok(s) = s.to_socket_addrs() {
            Ok(Self {
                original,
                addrs: s
                    .map(|a| Multiaddr::empty().with(a.ip().into()).with(Protocol::Tcp(a.port())))
                    .collect(),
            })
        } else if let Ok(s) = (s, 4458).to_socket_addrs() {
            Ok(Self {
                original,
                addrs: s
                    .map(|a| Multiaddr::empty().with(a.ip().into()).with(Protocol::Tcp(a.port())))
                    .collect(),
            })
        } else {
            Err(ActyxOSError::new(
                ActyxOSCode::ERR_INVALID_INPUT,
                format!("cannot interpret {} as address", original),
            ))
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct ConsoleOpt {
    #[structopt(name = "NODE", required = true)]
    /// the IP address or <host>:<admin port> of the node to perform the operation on.
    authority: Authority,
    #[structopt(short, long, value_name = "FILE")]
    /// File from which the identity (private key) for authentication is read.
    identity: Option<KeyPathWrapper>,
}

impl ConsoleOpt {
    pub async fn connect(&self) -> ActyxOSResult<(Sender<Task>, PeerId)> {
        let key = AxPrivateKey::try_from(&self.identity)?;
        let (task, mut channel) = mk_swarm(key).await?;
        tokio::spawn(task);
        let peer_id = connect(&mut channel, self.authority.clone()).await?;
        Ok((channel, peer_id))
    }
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

impl TryFrom<&Option<KeyPathWrapper>> for AxPrivateKey {
    type Error = ActyxOSError;
    fn try_from(k: &Option<KeyPathWrapper>) -> Result<AxPrivateKey> {
        if let Some(path) = k {
            AxPrivateKey::from_file(&path.0)
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
