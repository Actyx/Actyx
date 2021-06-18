use crate::cmd::{formats::Result, AxCliCommand, ConsoleOpt};
use futures::{stream, Stream, TryFutureExt};
use serde::Serialize;
use std::convert::TryInto;
use structopt::StructOpt;
use util::formats::{ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse};

pub struct SettingsGet();
impl AxCliCommand for SettingsGet {
    type Opt = GetOpt;
    type Output = serde_json::Value;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts).map_err(Into::into));
        Box::new(stream::once(r))
    }
    fn pretty(result: Self::Output) -> String {
        serde_yaml::to_string(&result).unwrap_or_else(|_| "Unkown error converting settings to yaml".into())
    }
}
#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// Gets settings for a specific scope.
pub struct GetOpt {
    #[structopt(flatten)]
    actual_opts: GetSettingsCommand,
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
}

#[derive(StructOpt, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GetSettingsCommand {
    #[structopt(long = "no-defaults")]
    /// Only return settings explicitly set by the user and skip default values.
    no_defaults: bool,
    #[structopt(name = "SCOPE", parse(try_from_str = super::parse_scope))]
    /// Scope from which you want to get the settings.
    scope: settings::Scope,
}

pub async fn run(mut opts: GetOpt) -> Result<serde_json::Value> {
    match opts
        .console_opt
        .authority
        .request(
            &opts.console_opt.identity.try_into()?,
            AdminRequest::SettingsGet {
                no_defaults: opts.actual_opts.no_defaults,
                scope: opts.actual_opts.scope,
            },
        )
        .await
    {
        Ok(AdminResponse::SettingsGetResponse(resp)) => Ok(resp),
        Ok(r) => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
        Err(err) => Err(err),
    }
}
