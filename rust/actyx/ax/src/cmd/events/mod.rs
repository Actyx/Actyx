mod dump;
mod offsets;
mod publish;
mod query;
mod restore;

use super::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = crate::util::version::VERSION.as_str())]
/// interact with the events API through the admin port
pub enum EventsOpts {
    #[structopt(no_version)]
    Offsets(offsets::OffsetsOpts),
    #[structopt(no_version)]
    Query(query::QueryOpts),
    #[structopt(no_version)]
    Publish(publish::PublishOpts),
    #[structopt(no_version)]
    Dump(dump::DumpOpts),
    #[structopt(no_version)]
    Restore(restore::RestoreOpts),
}

pub fn run(opts: EventsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        EventsOpts::Offsets(opt) => offsets::EventsOffsets::output(opt, json),
        EventsOpts::Query(opt) => query::EventsQuery::output(opt, json),
        EventsOpts::Publish(opt) => publish::EventsPublish::output(opt, json),
        EventsOpts::Dump(opt) => dump::EventsDump::output(opt, json),
        EventsOpts::Restore(opt) => restore::EventsRestore::output(opt, json),
    }
}
