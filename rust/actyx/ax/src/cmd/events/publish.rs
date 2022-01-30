use crate::{
    cmd::{AxCliCommand, ConsoleOpt},
    node_connection::Task,
};
use actyx_sdk::{
    service::{PublishEvent, PublishRequest, PublishResponse},
    Payload, Tag, TagSet,
};
use chrono::{DateTime, Utc};
use futures::{channel::mpsc::channel, future::ready, SinkExt, Stream, StreamExt};
use genawaiter::sync::Co;
use libp2p_streaming_response::v2::Response;
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

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(
            GenStream::new(move |co: Co<_>| async move {
                let tags = opts.tag.unwrap_or_default().into_iter().collect::<TagSet>();
                let payload = Payload::from_json_value(opts.payload)
                    .map_err(|msg| ActyxOSError::new(ActyxOSCode::ERR_INVALID_INPUT, msg))?;

                let mut tx = opts.console_opt.connect().await?;
                let (t, mut rx) = channel(2);
                tx.feed(Task::Events(
                    EventsRequest::Publish(PublishRequest {
                        data: vec![PublishEvent { tags, payload }],
                    }),
                    t,
                ))
                .await?;

                while let Some(x) = rx.next().await {
                    let msg = match x {
                        Response::Msg(msg) => msg,
                        Response::Error(e) => {
                            tracing::error!("{}", e);
                            return Err(ActyxOSError::new(ActyxOSCode::ERR_IO, e.to_string()));
                        }
                        Response::Finished => break,
                    };
                    match msg {
                        EventsResponse::Publish(res) => co.yield_(Ok(Some(res))).await,
                        EventsResponse::Error { message } => {
                            co.yield_(Err(ActyxOSError::new(ActyxOSCode::ERR_INVALID_INPUT, message)))
                                .await
                        }
                        _ => {}
                    }
                }
                Ok(None)
            })
            .filter_map(|x| ready(x.transpose())),
        )
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
