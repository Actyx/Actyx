use crate::cmd::{formats::Result, AxCliCommand, ConsoleOpt};
use futures::{stream, Stream, TryFutureExt};
use settings::Scope;
use std::{convert::TryInto, str::FromStr};
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

pub async fn run(mut opts: SchemaOpt) -> Result<serde_json::Value> {
    match opts
        .console_opt
        .authority
        .request(
            &opts.console_opt.identity.try_into()?,
            AdminRequest::SettingsSchema {
                scope: Scope::from_str("com.actyx")
                    .ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "cannot parse scope `/`")?,
            },
        )
        .await
    {
        Ok(AdminResponse::SettingsSchemaResponse(resp)) => Ok(resp),
        Ok(r) => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
        Err(err) => Err(err),
    }
}
