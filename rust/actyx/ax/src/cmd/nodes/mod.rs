mod ls;
use crate::cmd::AxCliCommand;
use futures::Future;
use ls::LsOpts;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
/// Manage nodes
pub enum NodesOpts {
    #[structopt(name = "ls")]
    /// Show node info and status
    Ls(LsOpts),
}

pub fn run(opts: NodesOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        NodesOpts::Ls(opt) => ls::NodesLs::output(opt, json),
    }
}
