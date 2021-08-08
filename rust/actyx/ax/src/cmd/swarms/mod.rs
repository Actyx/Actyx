pub mod keygen;

use crate::cmd::swarms::keygen::KeygenOpts;
use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// manage swarms
pub enum SwarmsOpts {
    #[structopt(no_version)]
    /// Generate a new swarm key.
    Keygen(KeygenOpts),
}

pub fn run(opts: SwarmsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        SwarmsOpts::Keygen(opt) => keygen::SwarmsKeygen::output(opt, json),
    }
}
