use crate::{
    cmd::{AxCliCommand, ConsoleOpt},
    node_connection::Task,
};
use actyx_sdk::{
    language::Query,
    service::{Diagnostic, EventResponse, Order, QueryRequest, Severity},
    Payload,
};
use futures::{channel::mpsc::channel, future::ready, SinkExt, Stream, StreamExt};
use libp2p_streaming_response::v2::Response;
use serde::{Deserialize, Serialize};
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
            let mut tx = opts.console_opt.connect().await?;
            let (t, mut rx) = channel(2);
            tx.feed(Task::Events(
                EventsRequest::Query(QueryRequest {
                    lower_bound: None,
                    upper_bound: None,
                    query: opts.query,
                    order: Order::Asc,
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
                    EventsResponse::Event(ev) => co.yield_(Ok(Some(EventDiagnostic::Event(ev)))).await,
                    EventsResponse::Diagnostic(d) => match d.severity {
                        Severity::Warning => co.yield_(Ok(Some(EventDiagnostic::Diagnostic(d)))).await,
                        Severity::Error => {
                            co.yield_(Err(ActyxOSError::new(ActyxOSCode::ERR_AQL_ERROR, d.message)))
                                .await
                        }
                        Severity::FutureCompat => {}
                    },
                    EventsResponse::Error { message } => {
                        co.yield_(Err(ActyxOSError::new(ActyxOSCode::ERR_INVALID_INPUT, message)))
                            .await
                    }
                    _ => {}
                }
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
