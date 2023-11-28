mod subscribe;
mod subscribe_monotonic;

use crate::cmd::AxCliCommand;
use futures::Future;

#[derive(clap::Subcommand, Clone, Debug)]
/// interact with the event API through the admin port
pub enum EventsOpts {
    Subscribe(subscribe::SubscribeOpts),
    SubscribeMonotonic(subscribe_monotonic::SubscribeMonotonicOpts),
}

pub fn run(opts: EventsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        EventsOpts::Subscribe(opt) => subscribe::EventsSubscribe::output(opt, json),
        EventsOpts::SubscribeMonotonic(opt) => subscribe_monotonic::EventsSubscribeMonotonic::output(opt, json),
    }
}
