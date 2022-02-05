use crate::{
    cmd::{formats::Result, AxCliCommand, ConsoleOpt},
    node_connection::{request_single, Task},
};
use futures::{stream, Stream, TryFutureExt};
use settings::Scope;
use std::str::FromStr;
use structopt::StructOpt;
use util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt, AdminRequest, AdminResponse};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// Gets a schema for a given scope.
pub struct SchemaOpt {
    #[structopt(flatten)]
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

pub async fn run(opts: SchemaOpt) -> Result<serde_json::Value> {
    let mut conn = opts.console_opt.connect().await?;
    let scope = Scope::from_str("com.actyx").ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "cannot parse scope `/`")?;
    request_single(
        &mut conn,
        |tx| Task::Admin(AdminRequest::SettingsSchema { scope }, tx),
        |m| match m {
            AdminResponse::SettingsSchemaResponse(resp) => Ok(resp),
            r => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
        },
    )
    .await
}
