use crate::cmd::{AxCliCommand, ConsoleOpt};
use actyx_sdk::{
    service::{Diagnostic, EventResponse, Order, QueryRequest, Severity},
    Payload,
};
use futures::{future::ready, Stream, StreamExt};
use runtime::value::Value;
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
    query: String,
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
            let mut conn = opts.console_opt.connect().await?;
            let mut s = conn
                .request_events(EventsRequest::Query(QueryRequest {
                    lower_bound: None,
                    upper_bound: None,
                    query: opts.query,
                    order: Order::Asc,
                }))
                .await?;

            while let Some(x) = s.next().await {
                match x {
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
        match result {
            EventDiagnostic::Event(e) => Value::from(e).to_string(),
            EventDiagnostic::Diagnostic(d) => format!("{:?}: {}", d.severity, d.message),
        }
    }
}
