use crate::cmd::{AxCliCommand, ConsoleOpt};
use ax_core::{
    node_connection::{request_single, Task},
    util::formats::{
        events_protocol::{EventsRequest, EventsResponse},
        ActyxOSError, ActyxOSResult,
    },
};
use ax_sdk::types::service::OffsetsResponse;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Table};
use futures::{stream, FutureExt, Stream};
use std::collections::BTreeSet;

#[derive(clap::Parser, Clone, Debug)]
/// obtain currently known offsets and replication targets
pub struct OffsetsOpts {
    #[command(flatten)]
    console_opt: ConsoleOpt,
}

pub struct EventsOffsets;
impl AxCliCommand for EventsOffsets {
    type Opt = OffsetsOpts;
    type Output = OffsetsResponse;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(stream::once(
            async move {
                let (mut conn, peer) = opts.console_opt.connect().await?;
                request_single(
                    &mut conn,
                    move |tx| Task::Events(peer, EventsRequest::Offsets, tx),
                    |response| match response {
                        EventsResponse::Offsets(o) => Ok(o),
                        x => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", x))),
                    },
                )
                .await
            }
            .boxed(),
        ))
    }

    fn pretty(result: Self::Output) -> String {
        let OffsetsResponse { present, to_replicate } = result;
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .set_header(["STREAM ID", "OFFSET", "TO REPLICATE"]);

        let streams = present
            .streams()
            .chain(to_replicate.keys().cloned())
            .collect::<BTreeSet<_>>();
        for s in streams.into_iter() {
            table.add_row([
                Cell::new(s),
                Cell::new(present.get(s).map(|x| x.to_string()).unwrap_or_default()),
                Cell::new(to_replicate.get(&s).map(|x| format!("+{}", *x)).unwrap_or_default()),
            ]);
        }
        table.to_string()
    }
}
