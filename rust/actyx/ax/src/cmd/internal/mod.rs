mod convert;
mod trees;
use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

use self::convert::ConvertFromV1Opts;
use self::trees::TreesOpts;

#[derive(StructOpt, Debug)]
#[structopt(no_version)]
pub enum InternalOpts {
    #[structopt(name = "convert")]
    /// Convert block
    ConvertFromV1(ConvertFromV1Opts),
    /// Interact with ax trees
    Trees(TreesOpts),
}

#[allow(dead_code)]
pub fn run(opts: InternalOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        InternalOpts::ConvertFromV1(opts) => convert::ConvertFromV1::output(opts, json),
        InternalOpts::Trees(opts) => trees::run(opts, json),
    }
}
