use crate::cmd::{consts::TABLE_FORMAT, AxCliCommand, ConsoleOpt};
use actyx_sdk::service::OffsetsResponse;
use futures::{stream, FutureExt, Stream, StreamExt};
use prettytable::{cell, row, Table};
use std::collections::BTreeSet;
use structopt::StructOpt;
use util::formats::{
    events_protocol::{EventsRequest, EventsResponse},
    ActyxOSError, ActyxOSResult,
};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// obtain currently known offsets and replication targets
pub struct OffsetsOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
}

pub struct EventsOffsets;
impl AxCliCommand for EventsOffsets {
    type Opt = OffsetsOpts;
    type Output = OffsetsResponse;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let fut = async move {
            let response = opts
                .console_opt
                .connect()
                .await?
                .request_events(EventsRequest::Offsets)
                .await?
                .next()
                .await;
            match response {
                Some(EventsResponse::Offsets(o)) => Ok(o),
                Some(x) => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", x))),
                None => Err(ActyxOSError::internal("stream closed without result".to_string())),
            }
        };
        Box::new(stream::once(fut.boxed()))
    }

    fn pretty(result: Self::Output) -> String {
        let OffsetsResponse { present, to_replicate } = result;
        let mut table = Table::new();
        table.set_format(*TABLE_FORMAT);
        table.set_titles(row!["STREAM ID", "OFFSET", "TO REPLICATE"]);
        let streams = present
            .streams()
            .chain(to_replicate.keys().cloned())
            .collect::<BTreeSet<_>>();
        for s in streams.into_iter() {
            table.add_row(row![
                s,
                present.get(s).map(|x| x.to_string()).unwrap_or_default(),
                to_replicate.get(&s).map(|x| format!("+{}", *x)).unwrap_or_default()
            ]);
        }
        table.to_string()
    }
}
