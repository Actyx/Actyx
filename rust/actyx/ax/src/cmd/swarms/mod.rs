pub mod keygen;

use crate::cmd::{swarms::keygen::KeygenOpts, AxCliCommand};
use futures::Future;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = crate::util::version::VERSION.as_str())]
/// manage swarms
pub enum SwarmsOpts {
    /// Generate a new swarm key.
    #[structopt(no_version)]
    Keygen(KeygenOpts),
}

pub fn run(opts: SwarmsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        SwarmsOpts::Keygen(opt) => keygen::SwarmsKeygen::output(opt, json),
    }
}
