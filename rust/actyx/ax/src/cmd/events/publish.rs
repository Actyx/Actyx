use crate::cmd::{AxCliCommand, ConsoleOpt};
use actyx_sdk::{
    service::{PublishEvent, PublishRequest, PublishResponse},
    Payload, Tag, TagSet,
};
use chrono::{DateTime, Utc};
use futures::{future::ready, stream, FutureExt, Stream, StreamExt};
use std::convert::TryInto;
use structopt::StructOpt;
use util::formats::{
    events_protocol::{EventsRequest, EventsResponse},
    ActyxOSCode, ActyxOSError, ActyxOSResult,
};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// publish an event
pub struct PublishOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
    /// event payload (JSON)
    payload: serde_json::Value,
    #[structopt(long, short)]
    /// tag (can be given multiple times)
    tag: Option<Vec<Tag>>,
}

pub struct EventsPublish;
impl AxCliCommand for EventsPublish {
    type Opt = PublishOpts;
    type Output = PublishResponse;

    fn run(mut opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let ret = async move {
            let tags = opts.tag.unwrap_or_default().into_iter().collect::<TagSet>();
            let payload = Payload::from_json_value(opts.payload)
                .map_err(|msg| ActyxOSError::new(ActyxOSCode::ERR_INVALID_INPUT, msg))?;
            opts.console_opt
                .authority
                .request_events(
                    &opts.console_opt.identity.try_into()?,
                    EventsRequest::Publish(PublishRequest {
                        data: vec![PublishEvent { tags, payload }],
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
                Ok(EventsResponse::Publish(res)) => Some(Ok(res)),
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
        use std::fmt::Write;

        let mut s = String::new();
        for key in result.data {
            let ts = DateTime::<Utc>::from(key.timestamp);
            writeln!(&mut s, "published event {}/{} at {}", key.stream, key.offset, ts).unwrap();
        }
        s
    }
}
