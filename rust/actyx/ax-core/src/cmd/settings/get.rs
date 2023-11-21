use crate::{
    cmd::{formats::Result, AxCliCommand, ConsoleOpt},
    node_connection::{request_single, Task},
    util::formats::{ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse},
};
use futures::{stream, Stream};
use serde::Serialize;
use structopt::StructOpt;

pub struct SettingsGet();
impl AxCliCommand for SettingsGet {
    type Opt = GetOpt;
    type Output = serde_json::Value;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts));
        Box::new(stream::once(r))
    }
    fn pretty(result: Self::Output) -> String {
        serde_yaml::to_string(&result).unwrap_or_else(|_| "Unkown error converting settings to yaml".into())
    }
}
#[derive(StructOpt, Debug)]
#[structopt(version = crate::util::version::VERSION.as_str())]
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
    /// Only return settings explicitly set by the user and skip default values.
    #[structopt(long = "no-defaults")]
    no_defaults: bool,
    /// Scope from which you want to get the settings.
    #[structopt(name = "SCOPE", parse(try_from_str = super::parse_scope))]
    scope: crate::settings::Scope,
}

pub async fn run(opts: GetOpt) -> Result<serde_json::Value> {
    let (mut conn, peer) = opts.console_opt.connect().await?;
    request_single(
        &mut conn,
        move |tx| {
            Task::Admin(
                peer,
                AdminRequest::SettingsGet {
                    no_defaults: opts.actual_opts.no_defaults,
                    scope: opts.actual_opts.scope,
                },
                tx,
            )
        },
        |t| match t {
            AdminResponse::SettingsGetResponse(resp) => Ok(resp),
            r => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
        },
    )
    .await
}
