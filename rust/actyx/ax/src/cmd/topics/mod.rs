mod delete;
mod ls;

use futures::Future;
use structopt::StructOpt;

use self::{
    delete::{DeleteOpts, TopicsDelete},
    ls::{LsOpts, TopicsList},
};

use super::{Authority, AxCliCommand};

/// manage topics
#[derive(StructOpt, Debug)]
#[structopt(version = ax_core::util::version::VERSION.as_str())]
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
