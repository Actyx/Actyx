mod sign;

use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

pub use sign::{create_signed_app_manifest, SignOpts};

#[derive(StructOpt, Debug)]
#[structopt(no_version)]
pub enum AppsOpts {
    /// Sign application manifest
    Sign(SignOpts),
}

pub fn run(opts: AppsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        AppsOpts::Sign(opt) => sign::AppsSign::output(opt, json),
    }
}
