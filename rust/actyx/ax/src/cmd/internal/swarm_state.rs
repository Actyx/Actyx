use std::convert::TryInto;

use crate::cmd::{AxCliCommand, ConsoleOpt};
use futures::{stream, FutureExt, Stream};
use structopt::StructOpt;
use util::formats::{ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse, InternalResponse};

#[derive(StructOpt, Debug)]
pub struct SwarmStateOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
}

pub struct SwarmState();
impl AxCliCommand for SwarmState {
    type Opt = SwarmStateOpts;
    type Output = serde_json::Value;
    fn run(mut opts: SwarmStateOpts) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let fut = async move {
            opts.console_opt.assert_local()?;
            let response = opts
                .console_opt
                .authority
                .request(
                    &opts.console_opt.identity.try_into()?,
                    AdminRequest::Internal(util::formats::InternalRequest::GetSwarmState),
                )
                .await;
            match response {
                Ok((_, AdminResponse::Internal(InternalResponse::GetSwarmStateResponse(resp)))) => Ok(resp),
                Ok(r) => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
                Err(err) => Err(err),
            }
        }
        .boxed();
        Box::new(stream::once(fut))
    }

    fn pretty(result: Self::Output) -> String {
        serde_json::to_string(&result).unwrap()
    }
}
