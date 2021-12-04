mod addkey;
mod keygen;

use crate::cmd::AxCliCommand;
use addkey::AddKeyOpts;
use futures::Future;
use keygen::KeygenOpts;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// manage user keys
pub enum UsersOpts {
    #[structopt(no_version)]
    /// Generate a new user key pair for interacting with an Actyx node.
    AddKey(AddKeyOpts),
    #[structopt(no_version)]
    /// Generate a new user key pair for interacting with an Actyx node.
    Keygen(KeygenOpts),
}

pub fn run(opts: UsersOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        UsersOpts::Keygen(opt) => keygen::UsersKeygen::output(opt, json),
        UsersOpts::AddKey(opt) => addkey::UsersAddKey::output(opt, json),
    }
}
