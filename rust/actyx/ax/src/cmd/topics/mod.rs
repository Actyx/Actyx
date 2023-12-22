mod delete;
mod ls;

use self::{
    delete::{DeleteOpts, TopicsDelete},
    ls::{LsOpts, TopicsList},
};
use super::AxCliCommand;
use futures::Future;

/// manage topics
#[derive(clap::Subcommand, Clone, Debug)]
pub enum TopicsOpts {
    Ls(LsOpts),
    Delete(DeleteOpts),
}

pub fn run(opts: TopicsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        TopicsOpts::Ls(opts) => TopicsList::output(opts, json),
        TopicsOpts::Delete(opts) => TopicsDelete::output(opts, json),
    }
}
