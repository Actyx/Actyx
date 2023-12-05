use crate::cmd::{AxCliCommand, ConsoleOpt};
use ax_core::{
    node_connection::{request_events, EventDiagnostic},
    util::{
        formats::{events_protocol::EventsRequest, ActyxOSResult},
        gen_stream::GenStream,
    },
};
use ax_sdk::types::{service::SubscribeMonotonicRequest, OffsetMap};
use futures::{future::ready, Stream, StreamExt};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = ax_core::util::version::VERSION.as_str())]
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
                    lower_bound: OffsetMap::default(),
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
