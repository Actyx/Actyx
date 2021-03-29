use std::convert::TryInto;

use crate::cmd::{formats::Result, AxCliCommand, ConsoleOpt};
use futures::{stream, Stream, TryFutureExt};
use structopt::StructOpt;
use util::formats::{ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse};

pub struct SettingsScopes();
impl AxCliCommand for SettingsScopes {
    type Opt = ScopesOpt;
    type Output = Vec<String>;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts).map_err(Into::into));
        Box::new(stream::once(r))
    }
    fn pretty(result: Self::Output) -> String {
        result.join("\n")
    }
}
#[derive(StructOpt, Debug)]
/// Gets available scopes from a node.
pub struct ScopesOpt {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
}

pub async fn run(mut opts: ScopesOpt) -> Result<Vec<String>> {
    opts.console_opt.assert_local()?;
    match opts
        .console_opt
        .authority
        .request(&opts.console_opt.identity.try_into()?, AdminRequest::SettingsScopes)
        .await
    {
        Ok((_, AdminResponse::SettingsScopesResponse(resp))) => Ok(resp),
        Ok(r) => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
        Err(err) => Err(err),
    }
}
