mod publish;
mod subscribe;
mod subscribe_monotonic;

use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// interact with the event API through the admin port
pub enum EventsOpts {
    #[structopt(no_version)]
    Subscribe(subscribe::SubscribeOpts),
    #[structopt(no_version)]
    SubscribeMonotonic(subscribe_monotonic::SubscribeMonotonicOpts),
    #[structopt(no_version)]
    Publish(publish::PublishOpts),
}

pub fn run(opts: EventsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        EventsOpts::Publish(opt) => publish::EventsPublish::output(opt, json),
        EventsOpts::Subscribe(opt) => subscribe::EventsSubscribe::output(opt, json),
        EventsOpts::SubscribeMonotonic(opt) => subscribe_monotonic::EventsSubscribeMonotonic::output(opt, json),
    }
}
