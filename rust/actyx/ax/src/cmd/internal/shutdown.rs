use crate::cmd::{AxCliCommand, ConsoleOpt};
use futures::{stream, FutureExt, Stream};
use structopt::StructOpt;
use util::formats::ActyxOSResult;

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// request the node to shut down
pub struct ShutdownOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
}

pub struct Shutdown;
impl AxCliCommand for Shutdown {
    type Opt = ShutdownOpts;
    type Output = String;
    fn run(opts: ShutdownOpts) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let fut = async move {
            opts.console_opt.connect().await?.shutdown().await?;
            Ok("shutdown request sent".to_string())
        }
        .boxed();
        Box::new(stream::once(fut))
    }

    fn pretty(result: Self::Output) -> String {
        result
    }
}
