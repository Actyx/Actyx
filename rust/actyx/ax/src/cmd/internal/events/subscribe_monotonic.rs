use crate::cmd::{AxCliCommand, ConsoleOpt};
use actyx_sdk::{
    language::Query,
    service::{EventResponse, StartFrom, SubscribeMonotonicRequest},
    OffsetMap, Payload,
};
use futures::{future::ready, stream, FutureExt, Stream, StreamExt};
use std::convert::TryInto;
use structopt::StructOpt;
use util::formats::{
    events_protocol::{EventsRequest, EventsResponse},
    ActyxOSCode, ActyxOSError, ActyxOSResult,
};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// obtain currently known offsets and replication targets
pub struct SubscribeMonotonicOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
    /// event API query
    query: Query,
}

pub struct EventsSubscribeMonotonic;
impl AxCliCommand for EventsSubscribeMonotonic {
    type Opt = SubscribeMonotonicOpts;
    type Output = EventResponse<Payload>;

    fn run(mut opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let ret = async move {
            opts.console_opt
                .authority
                .request_events(
                    &opts.console_opt.identity.try_into()?,
                    EventsRequest::SubscribeMonotonic(SubscribeMonotonicRequest {
                        session: "".into(),
                        from: StartFrom::LowerBound(OffsetMap::default()),
                        query: opts.query,
                    }),
                )
                .await
        }
        .boxed()
        .map(|x| match x {
            Ok(s) => s.map(Ok).left_stream(),
            Err(e) => stream::once(ready(Err(e))).right_stream(),
        })
        .flatten_stream()
        .filter_map(|x| {
            ready(match x {
                Ok(EventsResponse::Event(ev)) => Some(Ok(ev)),
                Ok(EventsResponse::Error { message }) => {
                    Some(Err(ActyxOSError::new(ActyxOSCode::ERR_INVALID_INPUT, message)))
                }
                Err(e) => Some(Err(e)),
                _ => None,
            })
        });
        Box::new(ret)
    }

    fn pretty(result: Self::Output) -> String {
        serde_json::to_string(&result).unwrap()
    }
}
