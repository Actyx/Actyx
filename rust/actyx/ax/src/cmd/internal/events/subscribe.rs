use crate::{
    cmd::{AxCliCommand, ConsoleOpt},
    node_connection::{request_events, EventDiagnostic},
};
use actyx_sdk::service::SubscribeRequest;
use futures::{future::ready, Stream, StreamExt};
use runtime::value::Value;
use std::{fs::File, io::Read};
use structopt::StructOpt;
use util::{
    formats::{events_protocol::EventsRequest, ActyxOSCode, ActyxOSResult, ActyxOSResultExt},
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
    type Output = EventDiagnostic;
    const WRAP: bool = false;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let ret = GenStream::new(move |co| async move {
            let query = if opts.query.starts_with('@') {
                let mut f = if opts.query == "@-" {
                    Box::new(std::io::stdin()) as Box<dyn Read>
                } else {
                    Box::new(File::open(&opts.query[1..]).ax_err(ActyxOSCode::ERR_IO)?)
                };
                let mut s = String::new();
                f.read_to_string(&mut s).ax_err(ActyxOSCode::ERR_IO)?;
                s
            } else {
                opts.query
            };
            let (mut conn, peer) = opts.console_opt.connect().await?;
            let mut stream = request_events(
                &mut conn,
                peer,
                EventsRequest::Subscribe(SubscribeRequest {
                    lower_bound: None,
                    query,
                }),
            )
            .await?;

            while let Some(ev) = stream.next().await {
                co.yield_(Ok(Some(ev?))).await;
            }
            Ok(None)
        })
        .filter_map(|x| ready(x.transpose()));
        Box::new(ret)
    }

    fn pretty(result: Self::Output) -> String {
        match result {
            EventDiagnostic::Event(e) => Value::from(e).to_string(),
            EventDiagnostic::AntiEvent(e) => format!("- {}", Value::from(e)),
            EventDiagnostic::Diagnostic(d) => format!("{:?}: {}", d.severity, d.message),
        }
    }
}
