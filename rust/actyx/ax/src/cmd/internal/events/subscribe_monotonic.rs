use crate::cmd::{AxCliCommand, ConsoleOpt};
use actyx_sdk::{
    service::{EventResponse, StartFrom, SubscribeMonotonicRequest},
    OffsetMap, Payload,
};
use futures::{future::ready, Stream, StreamExt};
use structopt::StructOpt;
use util::{
    formats::{
        events_protocol::{EventsRequest, EventsResponse},
        ActyxOSCode, ActyxOSError, ActyxOSResult,
    },
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
    type Output = EventResponse<Payload>;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let ret = GenStream::new(move |co| async move {
            let mut conn = opts.console_opt.connect().await?;
            let mut s = conn
                .request_events(EventsRequest::SubscribeMonotonic(SubscribeMonotonicRequest {
                    session: "".into(),
                    from: StartFrom::LowerBound(OffsetMap::default()),
                    query: opts.query,
                }))
                .await?;

            while let Some(x) = s.next().await {
                match x {
                    EventsResponse::Event(ev) => co.yield_(Ok(Some(ev))).await,
                    EventsResponse::Error { message } => {
                        return Err(ActyxOSError::new(ActyxOSCode::ERR_INVALID_INPUT, message))
                    }
                    _ => {}
                }
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
