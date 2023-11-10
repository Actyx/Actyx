mod events;
mod shutdown;
mod trees;

use self::{events::EventsOpts, shutdown::ShutdownOpts, trees::TreesOpts};
use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = crate::util::version::VERSION.as_str())]
/// do not use until instructed by Actyx
pub enum InternalOpts {
    /// Interact with ax trees
    #[structopt(no_version)]
    Trees(TreesOpts),
    /// Request the node to shut down
    #[structopt(no_version)]
    Shutdown(ShutdownOpts),
    /// Query the events API
    #[structopt(no_version)]
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
