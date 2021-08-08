mod offsets;
mod query;

use super::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// interact with the events API through the admin port
pub enum EventsOpts {
    #[structopt(no_version)]
    Offsets(offsets::OffsetsOpts),
    #[structopt(no_version)]
    Query(query::QueryOpts),
}

pub fn run(opts: EventsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        EventsOpts::Offsets(opt) => offsets::EventsOffsets::output(opt, json),
        EventsOpts::Query(opt) => query::EventsQuery::output(opt, json),
    }
}
