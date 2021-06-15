pub mod keygen;

use crate::cmd::swarms::keygen::KeygenOpts;
use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(no_version)]
pub enum SwarmsOpts {
    #[structopt(name = "keygen")]
    /// Generate a new swarm key.
    Keygen(KeygenOpts),
}

pub fn run(opts: SwarmsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        SwarmsOpts::Keygen(opt) => keygen::SwarmsKeygen::output(opt, json),
    }
}
