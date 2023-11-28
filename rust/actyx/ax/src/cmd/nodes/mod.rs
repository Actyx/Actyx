mod inspect;
mod ls;

use crate::cmd::AxCliCommand;
use futures::Future;
use inspect::InspectOpts;
use ls::LsOpts;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = ax_core::util::version::VERSION.as_str())]
/// get information about nodes
pub enum NodesOpts {
    /// Show node info and status
    #[structopt(no_version)]
    Ls(LsOpts),
    /// Show node details and connections
    #[structopt(no_version)]
    Inspect(InspectOpts),
}

pub fn run(opts: NodesOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        NodesOpts::Ls(opt) => ls::NodesLs::output(opt, json),
        NodesOpts::Inspect(opt) => inspect::NodesInspect::output(opt, json),
    }
}
