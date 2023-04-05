mod events;
mod shutdown;
mod trees;

use self::events::EventsOpts;
use self::shutdown::ShutdownOpts;
use self::trees::TreesOpts;
use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// do not use until instructed by Actyx
pub enum InternalOpts {
    #[structopt(no_version)]
    /// Interact with ax trees
    Trees(TreesOpts),
    #[structopt(no_version)]
    /// Request the node to shut down
    Shutdown(ShutdownOpts),
    #[structopt(no_version)]
    /// Query the events API
    Events(EventsOpts),
}

#[allow(dead_code)]
pub fn run(opts: InternalOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        InternalOpts::Events(opts) => events::run(opts, json),
        InternalOpts::Shutdown(opts) => shutdown::Shutdown::output(opts, json),
        InternalOpts::Trees(opts) => trees::run(opts, json),
    }
}
