use crate::{
    cmd::{AxCliCommand, ConsoleOpt},
    node_connection::request_events,
};
use actyx_sdk::{
    service::{Diagnostic, EventResponse, Order, QueryRequest},
    Payload,
};
use futures::{future::ready, Stream, StreamExt};
use runtime::value::Value;
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read};
use structopt::StructOpt;
use util::{
    formats::{events_protocol::EventsRequest, ActyxOSCode, ActyxOSResult, ActyxOSResultExt},
    gen_stream::GenStream,
};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// query the events API through the admin port
pub struct QueryOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
    /// event API query (read from file if the argument starts with @)
    query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventDiagnostic {
    Event(EventResponse<Payload>),
    Diagnostic(Diagnostic),
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
                co.yield_(Ok(Some(EventDiagnostic::Event(ev?)))).await;
            }
            Ok(None)
        })
        .filter_map(|x| ready(x.transpose()));
        Box::new(ret)
    }

    fn pretty(result: Self::Output) -> String {
        match result {
            EventDiagnostic::Event(e) => Value::from(e).to_string(),
            EventDiagnostic::Diagnostic(d) => format!("{:?}: {}", d.severity, d.message),
        }
    }
}
