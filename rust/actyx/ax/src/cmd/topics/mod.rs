mod ls;

use futures::Future;
use structopt::StructOpt;

use self::ls::{LsOpts, TopicsList};

use super::{Authority, AxCliCommand, KeyPathWrapper};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
pub enum TopicsOpts {
    #[structopt(no_version)]
    Ls(LsOpts),
}

pub fn run(opts: TopicsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        TopicsOpts::Ls(opts) => TopicsList::output(opts, json),
    }
}
