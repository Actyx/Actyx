mod delete;
mod ls;

use futures::Future;
use structopt::StructOpt;

use self::delete::{DeleteOpts, TopicsDelete};
use self::ls::{LsOpts, TopicsList};

use super::{Authority, AxCliCommand, KeyPathWrapper};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
pub enum TopicsOpts {
    #[structopt(no_version)]
    Ls(LsOpts),
    #[structopt(no_version)]
    Delete(DeleteOpts),
}

pub fn run(opts: TopicsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        TopicsOpts::Ls(opts) => TopicsList::output(opts, json),
        TopicsOpts::Delete(opts) => TopicsDelete::output(opts, json),
    }
}
