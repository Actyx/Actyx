mod inspect;
mod ls;

use crate::cmd::AxCliCommand;
use futures::Future;
use inspect::InspectOpts;
use ls::LsOpts;

#[derive(clap::Subcommand, Clone, Debug)]
/// get information about nodes
pub enum NodesOpts {
    /// Show node info and status
    Ls(LsOpts),
    /// Show node details and connections
    Inspect(InspectOpts),
}

pub fn run(opts: NodesOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        NodesOpts::Ls(opt) => ls::NodesLs::output(opt, json),
        NodesOpts::Inspect(opt) => inspect::NodesInspect::output(opt, json),
    }
}
