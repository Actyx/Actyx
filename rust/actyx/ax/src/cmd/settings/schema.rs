use crate::cmd::{formats::Result, AxCliCommand, ConsoleOpt};
use futures::{stream, Stream, TryFutureExt};
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, str::FromStr};
use structopt::StructOpt;
use util::formats::{ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    app_id: String,
    host: String,
    already_started: bool,
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
#[derive(StructOpt, Debug)]
/// Gets a schema for a given scope.
pub struct SchemaOpt {
    #[structopt(flatten)]
    actual_opts: SchemaCommand,
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
}

#[derive(StructOpt, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SchemaCommand {
    /// Scope from which you want to get the schema.
    #[structopt(name = "SCOPE", parse(try_from_str = settings::Scope::from_str))]
    scope: settings::Scope,
}

pub async fn run(mut opts: SchemaOpt) -> Result<serde_json::Value> {
    opts.console_opt.assert_local()?;
    match opts
        .console_opt
        .authority
        .request(
            &opts.console_opt.identity.try_into()?,
            AdminRequest::SettingsSchema {
                scope: opts.actual_opts.scope,
            },
        )
        .await
    {
        Ok(AdminResponse::SettingsSchemaResponse(resp)) => Ok(resp),
        Ok(r) => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
        Err(err) => Err(err),
    }
}
