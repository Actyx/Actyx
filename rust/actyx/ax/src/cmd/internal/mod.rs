mod convert;
mod swarm_state;
use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;
use swarm_state::SwarmStateOpts;

use self::convert::ConvertFromV1Opts;

#[derive(StructOpt, Debug)]
/// Manage ActyxOS nodes
pub enum InternalOpts {
    #[structopt(name = "swarm")]
    /// Show swarm and connectivity state
    SwarmState(SwarmStateOpts),
    #[structopt(name = "convert")]
    /// Convert block
    ConvertFromV1(ConvertFromV1Opts),
}

pub fn run(opts: InternalOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        InternalOpts::SwarmState(opts) => swarm_state::SwarmState::output(opts, json),
        InternalOpts::ConvertFromV1(opts) => convert::ConvertFromV1::output(opts, json),
    }
}
