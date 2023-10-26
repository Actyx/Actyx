use std::{fs::File, io::Read};

use crate::{
    cmd::{AxCliCommand, ConsoleOpt},
    node_connection::Task,
};
use actyx_sdk::{
    service::{PublishEvent, PublishRequest, PublishResponse},
    Payload, Tag, TagSet,
};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use futures::{channel::mpsc::channel, future::ready, SinkExt, Stream, StreamExt};
use genawaiter::sync::Co;
use structopt::StructOpt;
use util::{
    formats::{
        events_protocol::{EventsRequest, EventsResponse},
        ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt,
    },
    gen_stream::GenStream,
};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// publish an event
pub struct PublishOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,

    /// Event payload, needs to be valid JSON.
    /// You may also pass a file in using the syntax `@file.json` or
    /// have the command read from standard input using `@-`.
    payload: String,

    /// tag (can be given multiple times)
    #[structopt(long, short)]
    tag: Option<Vec<Tag>>,
}

fn load_json(input: String) -> ActyxOSResult<serde_json::Value> {
    let i = if input == "@-" {
        let stdin = std::io::stdin();
        let mut stdin = stdin.lock(); // locking is optional

        let mut line = String::new();
        while let Ok(n_bytes) = stdin.read_to_string(&mut line) {
            if n_bytes == 0 {
                break;
            }
        }
        serde_json::from_str(&line)
    } else if input.starts_with('@') {
        let f: &str = input
            .chars()
            .next()
            .map(|c| &input[c.len_utf8()..])
            .ok_or_else(|| anyhow!("Malformed input"))
            .ax_invalid_input()?;
        let manifest_file = File::open(f).ax_invalid_input()?;
        serde_json::from_reader(manifest_file)
    } else {
        serde_json::from_str(&input)
    };

    i.ax_invalid_input()
}

pub struct EventsPublish;
impl AxCliCommand for EventsPublish {
    type Opt = PublishOpts;
    type Output = PublishResponse;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(
            GenStream::new(move |co: Co<_>| async move {
                let tags = opts.tag.unwrap_or_default().into_iter().collect::<TagSet>();
                let payload = Payload::from_json_value(load_json(opts.payload)?)
                    .map_err(|msg| ActyxOSError::new(ActyxOSCode::ERR_INVALID_INPUT, msg))?;

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
