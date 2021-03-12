use std::convert::TryInto;

use crate::cmd::{AxCliCommand, ConsoleOpt};
use actyxos_lib::formats::logs::LogEvent;
use actyxos_lib::ActyxOSResult;
use futures::{stream, Stream, TryStreamExt};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct TailOpts {
    #[structopt(short = "n", long = "entries", default_value = "20")]
    /// Output the last <entries> entries
    entries: usize,
    #[structopt(long)]
    /// Get all log entries (overrides --entries)
    all_entries: bool,
    #[structopt(short = "f", long = "follow")]
    /// Keep running and output entries as they are created
    follow: bool,
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
}

fn pretty_message(log: LogEvent) -> String {
    format!(
        "{} {} {} {}",
        log.log_timestamp.to_string(),
        log.severity,
        log.log_name,
        log.message
    )
}

pub struct LogsTail();
impl AxCliCommand for LogsTail {
    type Opt = TailOpts;
    type Output = Vec<LogEvent>;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts));
        Box::new(Box::pin(stream::once(r).try_flatten()))
    }
    fn pretty(result: Self::Output) -> String {
        result.into_iter().map(pretty_message).collect::<Vec<_>>().join("\n")
    }
}

async fn run(mut opts: TailOpts) -> ActyxOSResult<impl Stream<Item = ActyxOSResult<Vec<LogEvent>>>> {
    opts.console_opt.assert_local()?;

    opts.console_opt
        .authority
        .stream_logs(
            &opts.console_opt.identity.try_into()?,
            opts.entries,
            opts.follow,
            opts.all_entries,
        )
        .await
}
