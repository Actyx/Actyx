use crate::cmd::{AxCliCommand, ConsoleOpt};
use ax_core::{
    node_connection::{request_single, Task},
    settings::Scope,
    util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt, AdminRequest, AdminResponse},
};
use futures::{stream, Stream, TryFutureExt};
use std::str::FromStr;

#[derive(clap::Parser, Clone, Debug)]
/// Gets a schema for a given scope.
pub struct SchemaOpt {
    #[command(flatten)]
    console_opt: ConsoleOpt,
}

pub struct SettingsSchema();
impl AxCliCommand for SettingsSchema {
    type Opt = SchemaOpt;
    type Output = serde_json::Value;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts).map_err(Into::into));
        Box::new(stream::once(r))
    }
    fn pretty(result: Self::Output) -> String {
        serde_json::to_string(&result).unwrap_or_else(|_| "Unkown error converting schema to json".into())
    }
}

pub async fn run(opts: SchemaOpt) -> ActyxOSResult<serde_json::Value> {
    let (mut conn, peer) = opts.console_opt.connect().await?;
    let scope = Scope::from_str("com.actyx").ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "cannot parse scope `/`")?;
    request_single(
        &mut conn,
        move |tx| Task::Admin(peer, AdminRequest::SettingsSchema { scope }, tx),
        |m| match m {
            AdminResponse::SettingsSchemaResponse(resp) => Ok(resp),
            r => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
        },
    )
    .await
}
