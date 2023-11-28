use crate::cmd::{AxCliCommand, ConsoleOpt};
use ax_core::{
    node_connection::{request_single, Task},
    util::formats::{ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse},
};
use futures::{stream, Stream, TryFutureExt};
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    scope: String,
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
    scope: ax_core::settings::Scope,
}

#[derive(StructOpt, Debug)]
#[structopt(version = ax_core::util::version::VERSION.as_str())]
pub struct UnsetOpt {
    #[structopt(flatten)]
    actual_opts: UnsetSettingsCommand,
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
}

#[derive(StructOpt, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnsetSettingsCommand {
    /// Scope for which you want to unset the settings; use `/` for the root scope.
    #[structopt(name = "SCOPE", parse(try_from_str = super::parse_scope))]
    scope: ax_core::settings::Scope,
}

pub async fn run(opts: UnsetOpt) -> ActyxOSResult<Output> {
    let scope = opts.actual_opts.scope.clone();
    let (mut conn, peer) = opts.console_opt.connect().await?;
    request_single(
        &mut conn,
        move |tx| {
            Task::Admin(
                peer,
                AdminRequest::SettingsUnset {
                    scope: opts.actual_opts.scope,
                },
                tx,
            )
        },
        move |m| match m {
            AdminResponse::SettingsUnsetResponse => Ok(Output {
                scope: super::print_scope(scope.clone()),
            }),
            r => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
        },
    )
    .await
}
