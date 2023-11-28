mod dump;
mod offsets;
mod publish;
mod query;
mod restore;

use super::AxCliCommand;
use futures::Future;

#[derive(clap::Parser, Clone, Debug)]
/// interact with the events API through the admin port
pub enum EventsOpts {
    Offsets(offsets::OffsetsOpts),
    Query(query::QueryOpts),
    Publish(publish::PublishOpts),
    Dump(dump::DumpOpts),
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
