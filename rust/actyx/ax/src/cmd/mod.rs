use ax_core::authority::Authority;
use ax_core::private_key::KeyPathWrapper;
use ax_core::util::formats::ActyxOSResult;
use ax_core::{
    node_connection::{connect, mk_swarm, Task},
    private_key::AxPrivateKey,
};
use futures::{channel::mpsc::Sender, future, Future, Stream, StreamExt};
use libp2p::PeerId;
use serde::Serialize;
use structopt::StructOpt;

pub mod apps;
pub mod events;
mod formats;
pub mod internal;
pub mod nodes;
pub mod settings;
pub mod swarms;
pub mod topics;
pub mod users;

pub use formats::ActyxCliResult;

#[derive(StructOpt, Debug)]
pub struct ConsoleOpt {
    /// the IP address or `<host>:<admin port>` of the node to perform the operation on.
    #[structopt(name = "NODE", required = true)]
    authority: Authority,
    /// File from which the identity (private key) for authentication is read.
    #[structopt(short, long, value_name = "FILE_OR_KEY", env = "AX_IDENTITY", hide_env_values = true)]
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
