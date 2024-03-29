use crate::cmd::{AxCliCommand, ConsoleOpt};
use ax_core::{
    node_connection::{request, Task},
    util::formats::{ActyxOSCode, ActyxOSResult, AdminRequest},
};
use futures::{stream, FutureExt, Stream};

#[derive(clap::Parser, Clone, Debug)]
/// request the node to shut down
pub struct ShutdownOpts {
    #[command(flatten)]
    console_opt: ConsoleOpt,
}

pub struct Shutdown;
impl AxCliCommand for Shutdown {
    type Opt = ShutdownOpts;
    type Output = String;
    fn run(opts: ShutdownOpts) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let fut = async move {
            let (mut conn, peer) = opts.console_opt.connect().await?;
            let v = request(
                &mut conn,
                move |tx| Task::Admin(peer, AdminRequest::NodesShutdown, tx),
                |x| x,
            )
            .await?;
            if !v.is_empty() {
                Err(ActyxOSCode::ERR_INTERNAL_ERROR.with_message(format!("unexpected responses: {:?}", v)))
            } else {
                Ok("shutdown request sent".to_string())
            }
        }
        .boxed();
        Box::new(stream::once(fut))
    }

    fn pretty(result: Self::Output) -> String {
        result
    }
}
