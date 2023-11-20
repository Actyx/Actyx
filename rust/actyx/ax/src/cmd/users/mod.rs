mod addkey;
mod devcert;
mod keygen;
mod pubkey;

use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

use addkey::AddKeyOpts;
use devcert::DevCertOpts;
use keygen::KeygenOpts;
use pubkey::PubkeyOpts;

#[derive(StructOpt, Debug)]
#[structopt(version = crate::util::version::VERSION.as_str())]
/// manage user keys
pub enum UsersOpts {
    /// Install a user key into /admin/authorizedUsers of a local Actyx node that is not currently running.
    #[structopt(no_version)]
    AddKey(AddKeyOpts),
    /// Generate a new user key pair for interacting with an Actyx node.
    #[structopt(no_version)]
    Keygen(KeygenOpts),
    /// Show public key corresponding to a private key.
    #[structopt(no_version)]
    Pubkey(PubkeyOpts),
    /// Generate a new developer certificate.
    #[structopt(no_version)]
    DevCert(DevCertOpts),
}

pub fn run(opts: UsersOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        UsersOpts::Keygen(opt) => keygen::UsersKeygen::output(opt, json),
        UsersOpts::AddKey(opt) => addkey::UsersAddKey::output(opt, json),
        UsersOpts::DevCert(opt) => devcert::UsersDevCert::output(opt, json),
        UsersOpts::Pubkey(opt) => pubkey::UsersPubkey::output(opt, json),
    }
}
