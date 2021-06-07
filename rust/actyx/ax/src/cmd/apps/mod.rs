mod sign;
use crate::cmd::AxCliCommand;
use futures::Future;
use sign::SignOpts;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
/// Manage apps
pub enum AppsOpts {
    /// Sign application manifest
    Sign(SignOpts),
}

pub fn run(opts: AppsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        AppsOpts::Sign(opt) => sign::AppsSign::output(opt, json),
    }
}
