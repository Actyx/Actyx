mod keygen;

use crate::cmd::users::keygen::KeygenOpts;
use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
/// Manage ActyxOS swarms
pub enum UsersOpts {
    #[structopt(name = "keygen")]
    /// Generate a new user key pair for interacting with an Actyx node.
    Keygen(KeygenOpts),
}

pub fn run(opts: UsersOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        UsersOpts::Keygen(opt) => keygen::UsersKeygen::output(opt, json),
    }
}
