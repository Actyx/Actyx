mod tail;
use crate::cmd::logs::tail::TailOpts;
use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
/// Manage node and app logs
pub enum LogsOpts {
    #[structopt(name = "tail")]
    /// Get logs from a node
    Tail(TailOpts),
}

pub fn run(opts: LogsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        LogsOpts::Tail(opt) => tail::LogsTail::output(opt, json),
    }
}
