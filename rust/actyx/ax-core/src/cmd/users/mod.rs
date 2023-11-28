mod addkey;
mod devcert;
mod keygen;
mod pubkey;

use crate::cmd::AxCliCommand;
use futures::Future;

use addkey::AddKeyOpts;
use devcert::DevCertOpts;
use keygen::KeygenOpts;
use pubkey::PubkeyOpts;

#[derive(clap::Parser, Clone, Debug)]
/// manage user keys
pub enum UsersOpts {
    /// Install a user key into /admin/authorizedUsers of a local Actyx node that is not currently running.
    AddKey(AddKeyOpts),
    /// Generate a new user key pair for interacting with an Actyx node.
    Keygen(KeygenOpts),
    /// Show public key corresponding to a private key.
    Pubkey(PubkeyOpts),
    /// Generate a new developer certificate.
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
