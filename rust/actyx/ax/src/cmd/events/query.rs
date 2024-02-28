use crate::cmd::{AxCliCommand, ConsoleOpt};
use ax_core::{
    node_connection::{request_events, EventDiagnostic},
    runtime::value::Value,
    util::{
        formats::{events_protocol::EventsRequest, ActyxOSCode, ActyxOSResult, ActyxOSResultExt},
        gen_stream::GenStream,
    },
};
use ax_sdk::types::service::{Order, QueryRequest};
use futures::{future::ready, Stream, StreamExt};
use itertools::Itertools;
use std::{fs::File, io::Read};

#[derive(clap::Parser, Clone, Debug)]
/// query the events API through the admin port
pub struct QueryOpts {
    /// AQL features to enable
    #[arg(short, long)]
    features: Vec<String>,
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
            let mut query = if opts.query.starts_with('@') {
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
            if !opts.features.is_empty() {
                query.insert_str(
                    0,
                    &format!(
                        "PRAGMA features := {}\n",
                        opts.features
                            .into_iter()
                            .flat_map(|f| f.split(',').map(str::to_owned).collect::<Vec<_>>())
                            .join(" ")
                    ),
                );
            }
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
