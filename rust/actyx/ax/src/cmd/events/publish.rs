use crate::cmd::{AxCliCommand, ConsoleOpt};
use ax_core::{
    node_connection::Task,
    util::{
        formats::{
            events_protocol::{EventsRequest, EventsResponse},
            ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt,
        },
        gen_stream::GenStream,
    },
};
use ax_sdk::{
    service::{PublishEvent, PublishRequest, PublishResponse},
    Payload, Tag, TagSet,
};
use chrono::{DateTime, Utc};
use futures::{channel::mpsc::channel, future::ready, SinkExt, Stream, StreamExt};
use genawaiter::sync::Co;
use std::{fs::File, io::Read};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = ax_core::util::version::VERSION.as_str())]
/// publish an event
pub struct PublishOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,

    /// event payload (JSON) or @FILE for reading from a file (@- for stdin)
    payload: String,

    /// tag (can be given multiple times)
    #[structopt(long, short)]
    tag: Option<Vec<Tag>>,
}

/// Read the event:
/// - Read from standard input if it starts with `@-`
/// - Read from a file if it starts with a `@` instead
/// - Otherwise, take the parameter at face value
fn payload_from_opts(opts_payload: String) -> ActyxOSResult<Payload> {
    let mut contents = String::new();
    if let Some(stripped) = opts_payload.strip_prefix('@') {
        if stripped == "-" {
            let stdin = std::io::stdin();
            let mut stdin = stdin.lock(); // locking is optional
            stdin.read_to_string(&mut contents).ax_err(ActyxOSCode::ERR_IO)?;
        } else {
            File::open(&opts_payload[1..])
                .and_then(|mut file| file.read_to_string(&mut contents))
                .ax_err(ActyxOSCode::ERR_IO)?;
        }
    } else {
        contents = opts_payload
    }
    Payload::from_json_str(&contents).ax_invalid_input()
}

pub struct EventsPublish;
impl AxCliCommand for EventsPublish {
    type Opt = PublishOpts;
    type Output = PublishResponse;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(
            GenStream::new(move |co: Co<_>| async move {
                let tags = opts.tag.unwrap_or_default().into_iter().collect::<TagSet>();
                let payload = payload_from_opts(opts.payload)?;

                let (mut conn, peer) = opts.console_opt.connect().await?;
                let (tx, mut rx) = channel(2);
                conn.feed(Task::Events(
                    peer,
                    EventsRequest::Publish(PublishRequest {
                        data: vec![PublishEvent { tags, payload }],
                    }),
                    tx,
                ))
                .await?;

                while let Some(msg) = rx.next().await {
                    match msg? {
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
