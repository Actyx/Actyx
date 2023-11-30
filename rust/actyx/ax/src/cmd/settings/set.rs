use crate::cmd::{AxCliCommand, ConsoleOpt};
use ax_core::{
    node_connection::{request_single, Task},
    util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt, AdminRequest, AdminResponse},
};
use futures::{stream, Stream, TryFutureExt};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read};
use tracing::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    scope: String,
    settings: serde_json::Value,
}
pub struct SettingsSet();
impl AxCliCommand for SettingsSet {
    type Opt = SetOpt;
    type Output = Output;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts).map_err(Into::into));
        Box::new(stream::once(r))
    }
    fn pretty(result: Self::Output) -> String {
        serde_yaml::to_string(&result.settings)
            .map(|settings| {
                format!(
                    "Successfully replaced settings at {}. Created object with defaults:\n{}",
                    result.scope, settings
                )
            })
            .unwrap_or_else(|_| "Unknown error translating set settings to yaml".into())
    }
}

#[derive(clap::Parser, Clone, Debug)]
pub struct SetOpt {
    #[command(flatten)]
    actual_opts: SetSettingsCommand,
    #[command(flatten)]
    console_opt: ConsoleOpt,
}

#[derive(clap::Parser, Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetSettingsCommand {
    /// Scope for which you want to set the given settings; use `/` for the the root scope.
    #[arg(name = "SCOPE", value_parser = super::parse_scope)]
    scope: ax_core::settings::Scope,
    /// The value you want to set for the given scope as a YAML or JSON string.
    /// You may also pass in a file using the syntax `@file.yml` or have the
    /// command read from stdin using `@-`.
    #[arg(name = "VALUE")]
    input: String,
}

fn load_yml(input: String) -> ActyxOSResult<serde_yaml::Value> {
    if let Some(stripped) = input.strip_prefix('@') {
        if stripped == "-" {
            let stdin = std::io::stdin();
            let mut stdin = stdin.lock(); // locking is optional

            let mut line = String::new();
            stdin.read_to_string(&mut line).ax_err(ActyxOSCode::ERR_IO)?;
            serde_yaml::from_str(&line)
        } else {
            let manifest_file = File::open(&input[1..]).ax_err(ActyxOSCode::ERR_IO)?;
            serde_yaml::from_reader(manifest_file)
        }
    } else {
        serde_yaml::from_str(&input)
    }
    .ax_invalid_input()
}

pub async fn run(opts: SetOpt) -> ActyxOSResult<Output> {
    let settings = load_yml(opts.actual_opts.input)?;
    info!("Parsed {:?}", settings);
    let scope = opts.actual_opts.scope.clone();
    let scope2 = scope.clone();
    let json = serde_json::to_value(settings).ax_err_ctx(
        ax_core::util::formats::ActyxOSCode::ERR_INTERNAL_ERROR,
        "cannot parse provided value",
    )?;
    let (mut conn, peer) = opts.console_opt.connect().await?;
    request_single(
        &mut conn,
        move |tx| {
            Task::Admin(
                peer,
                AdminRequest::SettingsSet {
                    scope,
                    json,
                    ignore_errors: false,
                },
                tx,
            )
        },
        move |m| match m {
            AdminResponse::SettingsSetResponse(settings) => Ok(Output {
                scope: super::print_scope(scope2.clone()),
                settings,
            }),
            r => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
        },
    )
    .await
}
