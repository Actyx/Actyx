use crate::{
    cmd::{AxCliCommand, ConsoleOpt},
    node_connection::{request_events, EventDiagnostic},
    runtime::value::Value,
    util::{
        formats::{events_protocol::EventsRequest, ActyxOSCode, ActyxOSResult, ActyxOSResultExt},
        gen_stream::GenStream,
    },
};
use ax_sdk::service::{Order, QueryRequest};
use futures::{future::ready, Stream, StreamExt};
use std::{fs::File, io::Read};

#[derive(clap::Parser, Clone, Debug)]
/// query the events API through the admin port
pub struct QueryOpts {
    #[command(flatten)]
    console_opt: ConsoleOpt,
    /// event API query (read from file if the argument starts with @)
    query: String,
}

pub struct EventsQuery;
impl AxCliCommand for EventsQuery {
    type Opt = QueryOpts;
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
                EventsRequest::Query(QueryRequest {
                    lower_bound: None,
                    upper_bound: None,
                    query,
                    order: Order::Asc,
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
