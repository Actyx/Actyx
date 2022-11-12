use crate::{
    cmd::{AxCliCommand, ConsoleOpt},
    node_connection::request_events,
};
use actyx_sdk::{
    service::{EventResponse, SubscribeRequest},
    Payload,
};
use futures::{future::ready, Stream, StreamExt};
use structopt::StructOpt;
use util::{
    formats::{events_protocol::EventsRequest, ActyxOSResult},
    gen_stream::GenStream,
};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// subscribe to events
pub struct SubscribeOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
    /// event API query
    query: String,
}

pub struct EventsSubscribe;
impl AxCliCommand for EventsSubscribe {
    type Opt = SubscribeOpts;
    type Output = EventResponse<Payload>;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let ret = GenStream::new(move |co| async move {
            let (mut conn, peer) = opts.console_opt.connect().await?;
            let mut s = request_events(
                &mut conn,
                peer,
                EventsRequest::Subscribe(SubscribeRequest {
                    lower_bound: None,
                    query: opts.query,
                }),
            )
            .await?;

            while let Some(ev) = s.next().await {
                co.yield_(Ok(Some(ev?))).await;
            }

            Ok(None)
        })
        .filter_map(|x| ready(x.transpose()));
        Box::new(ret)
    }

    fn pretty(result: Self::Output) -> String {
        serde_json::to_string(&result).unwrap()
    }
}
