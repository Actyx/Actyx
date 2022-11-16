use crate::{
    cmd::{AxCliCommand, ConsoleOpt},
    node_connection::{request_events, EventDiagnostic},
};
use actyx_sdk::{
    service::{StartFrom, SubscribeMonotonicRequest},
    OffsetMap,
};
use futures::{future::ready, Stream, StreamExt};
use structopt::StructOpt;
use util::{
    formats::{events_protocol::EventsRequest, ActyxOSResult},
    gen_stream::GenStream,
};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// issue a monotonic subscription
pub struct SubscribeMonotonicOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
    /// event API query
    query: String,
}

pub struct EventsSubscribeMonotonic;
impl AxCliCommand for EventsSubscribeMonotonic {
    type Opt = SubscribeMonotonicOpts;
    type Output = EventDiagnostic;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let ret = GenStream::new(move |co| async move {
            let (mut conn, peer) = opts.console_opt.connect().await?;
            let mut s = request_events(
                &mut conn,
                peer,
                EventsRequest::SubscribeMonotonic(SubscribeMonotonicRequest {
                    session: "".into(),
                    from: StartFrom::LowerBound(OffsetMap::default()),
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
