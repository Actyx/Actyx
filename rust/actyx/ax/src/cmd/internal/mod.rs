mod events;
mod shutdown;
mod trees;

use self::{events::EventsOpts, shutdown::ShutdownOpts, trees::TreesOpts};
use crate::cmd::AxCliCommand;
use futures::Future;

#[derive(clap::Subcommand, Debug, Clone)]
/// do not use until instructed by Actyx
pub enum InternalOpts {
    /// Interact with ax trees
    #[command(subcommand)]
    Trees(TreesOpts),
    /// Request the node to shut down
    Shutdown(ShutdownOpts),
    /// Query the events API
    #[command(subcommand)]
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
