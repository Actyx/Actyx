pub mod keygen;

use crate::cmd::{swarms::keygen::KeygenOpts, AxCliCommand};
use futures::Future;

#[derive(clap::Subcommand, Clone, Debug)]
/// manage swarms
pub enum SwarmsOpts {
    /// Generate a new swarm key.
    Keygen(KeygenOpts),
}

pub fn run(opts: SwarmsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        SwarmsOpts::Keygen(opt) => keygen::SwarmsKeygen::output(opt, json),
    }
}
