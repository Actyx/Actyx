use crate::{
    cmd::{AxCliCommand, ConsoleOpt},
    node_connection::request_events,
};
use actyx_sdk::{
    language::Query,
    service::{Diagnostic, EventResponse, Order, QueryRequest},
    Payload,
};
use futures::{future::ready, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use util::{
    formats::{events_protocol::EventsRequest, ActyxOSResult},
    gen_stream::GenStream,
};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// query the events API through the admin port
pub struct QueryOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
    /// event API query
    query: Query,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventDiagnostic {
    Event(EventResponse<Payload>),
    Diagnostic(Diagnostic),
}

pub struct EventsQuery;
impl AxCliCommand for EventsQuery {
    type Opt = QueryOpts;
    type Output = EventDiagnostic;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let ret = GenStream::new(move |co| async move {
            let (mut conn, peer) = opts.console_opt.connect().await?;
            let mut stream = request_events(
                &mut conn,
                peer,
                EventsRequest::Query(QueryRequest {
                    lower_bound: None,
                    upper_bound: None,
                    query: opts.query,
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
        serde_json::to_string(&result).unwrap()
    }
}
