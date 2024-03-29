pub mod apps;
pub mod events;
pub mod internal;
pub mod nodes;
pub mod run;
pub mod settings;
pub mod swarms;
pub mod topics;
pub mod users;

use ax_core::{
    authority::Authority,
    node_connection::{connect, mk_swarm, Task},
    private_key::AxPrivateKey,
    util::formats::{ActyxOSError, ActyxOSResult},
};
use futures::{channel::mpsc::Sender, future, Future, Stream, StreamExt};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum ActyxCliResult<T> {
    OK { code: String, result: T },
    ERROR(ActyxOSError),
}
const OK: &str = "OK";
impl<T> From<ActyxOSResult<T>> for ActyxCliResult<T> {
    fn from(res: ActyxOSResult<T>) -> Self {
        match res {
            Ok(result) => ActyxCliResult::OK {
                code: OK.to_owned(),
                result,
            },
            Err(err) => ActyxCliResult::ERROR(err),
        }
    }
}

#[derive(clap::Parser, Clone, Debug)]
pub struct ConsoleOpt {
    /// the IP address or `<host>:<admin port>` of the node to perform the operation on.
    #[arg(name = "NODE", required = true)]
    authority: Authority,
    /// Authentication identity (private key).
    /// Can be base64 encoded or a path to a file containing the key,
    /// defaults to `<OS_CONFIG_FOLDER>/key/users/id`.
    #[arg(short, long, value_name = "FILE_OR_KEY", env = "AX_IDENTITY", hide_env_values = true)]
    identity: Option<String>,
}

pub(crate) fn load_identity(identity: &Option<String>) -> ActyxOSResult<AxPrivateKey> {
    if let Some(identity) = identity {
        AxPrivateKey::from_str(identity).or_else(|_| AxPrivateKey::from_file(identity))
    } else {
        AxPrivateKey::load_from_default_path()
    }
}

impl ConsoleOpt {
    pub async fn connect(&self) -> ActyxOSResult<(Sender<Task>, PeerId)> {
        let key = load_identity(&self.identity)?;
        let (task, mut channel) = mk_swarm(key).await?;
        tokio::spawn(task);
        let peer_id = connect(&mut channel, self.authority.clone()).await?;
        Ok((channel, peer_id))
    }
}

pub trait AxCliCommand {
    type Opt: clap::Parser;
    type Output: Serialize + 'static;
    const WRAP: bool = true;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin>;
    fn pretty(result: Self::Output) -> String;
    fn output(opts: Self::Opt, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
        Box::new(Self::run(opts).for_each(move |item| {
            let exit = if item.is_ok() { 0 } else { 1 };
            if json {
                if Self::WRAP {
                    println!(
                        "{}",
                        serde_json::to_string(&ActyxCliResult::<Self::Output>::from(item)).unwrap()
                    );
                } else {
                    let item = match item {
                        Ok(item) => serde_json::to_string(&item).unwrap(),
                        Err(e) => serde_json::to_string(&ActyxCliResult::<Self::Output>::from(Err(e))).unwrap(),
                    };
                    println!("{}", item);
                }
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

pub(crate) fn determine_ax_default_data_dir() -> anyhow::Result<std::path::PathBuf> {
    use anyhow::Context;
    let cwd = std::env::current_dir().context("getting current working directory")?;

    Ok({
        let actyx_data = cwd.join("actyx-data");
        if actyx_data.exists() {
            eprintln!(
                concat!(
                    "Warning: the `actyx-data` directory has been deprecated. ",
                    "If you want to get rid of this warning, rename `{0}/actyx-data` to `{0}/ax-data`."
                ),
                cwd.display()
            );
            actyx_data
        } else {
            cwd.join("ax-data")
        }
    })
}
