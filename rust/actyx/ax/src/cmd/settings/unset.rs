use crate::cmd::{formats::Result, AxCliCommand, ConsoleOpt};
use futures::{stream, Stream, TryFutureExt};
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, str::FromStr};
use structopt::StructOpt;
use util::formats::{ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    scope: axossettings::Scope,
}

pub struct SettingsUnset();
impl AxCliCommand for SettingsUnset {
    type Opt = UnsetOpt;
    type Output = Output;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts).map_err(Into::into));
        Box::new(stream::once(r))
    }
    fn pretty(result: Self::Output) -> String {
        format!("Successfully unset settings at {}.", result.scope)
    }
}
#[derive(Serialize)]
struct RequestBody {
    scope: axossettings::Scope,
}

#[derive(StructOpt, Debug)]
pub struct UnsetOpt {
    #[structopt(flatten)]
    actual_opts: UnsetSettingsCommand,
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
}

#[derive(StructOpt, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnsetSettingsCommand {
    #[structopt(name = "SCOPE", parse(try_from_str = axossettings::Scope::from_str))]
    /// Scope for which you want to unset the settings.
    scope: axossettings::Scope,
}

pub async fn run(mut opts: UnsetOpt) -> Result<Output> {
    opts.console_opt.assert_local()?;
    let scope = opts.actual_opts.scope.clone();

    match opts
        .console_opt
        .authority
        .request(
            &opts.console_opt.identity.try_into()?,
            AdminRequest::SettingsUnset {
                scope: opts.actual_opts.scope,
            },
        )
        .await
    {
        Ok((_, AdminResponse::SettingsUnsetResponse)) => Ok(Output { scope }),
        Ok(r) => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
        Err(err) => Err(err),
    }
}
