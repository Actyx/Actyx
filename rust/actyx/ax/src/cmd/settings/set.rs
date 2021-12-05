use crate::cmd::{formats::Result, AxCliCommand, ConsoleOpt};
use anyhow::anyhow;
use futures::{stream, Stream, TryFutureExt};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read};
use structopt::StructOpt;
use tracing::*;
use util::formats::{ActyxOSError, ActyxOSResult, ActyxOSResultExt, AdminRequest, AdminResponse};

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

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
pub struct SetOpt {
    #[structopt(flatten)]
    actual_opts: SetSettingsCommand,
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
}

#[derive(StructOpt, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetSettingsCommand {
    #[structopt(name = "SCOPE", parse(try_from_str = super::parse_scope))]
    /// Scope for which you want to set the given settings; use `/` for the the root scope.
    scope: settings::Scope,
    #[structopt(name = "VALUE")]
    /// The value you want to set for the given scope as a YAML or JSON string.
    /// You may also pass in a file using the syntax `@file.yml` or have the
    /// command read from stdin using `@-`.
    input: String,
}

fn load_yml(input: String) -> Result<serde_yaml::Value> {
    let i = if input == "@-" {
        let stdin = std::io::stdin();
        let mut stdin = stdin.lock(); // locking is optional

        let mut line = String::new();
        while let Ok(n_bytes) = stdin.read_to_string(&mut line) {
            if n_bytes == 0 {
                break;
            }
        }
        serde_yaml::from_str(&line)
    } else if input.starts_with('@') {
        let f: &str = input
            .chars()
            .next()
            .map(|c| &input[c.len_utf8()..])
            .ok_or_else(|| anyhow!("Malformed input"))
            .ax_invalid_input()?;
        let manifest_file = File::open(f).ax_invalid_input()?;
        serde_yaml::from_reader(manifest_file)
    } else {
        serde_yaml::from_str(&input)
    };

    i.ax_invalid_input()
}

pub async fn run(opts: SetOpt) -> Result<Output> {
    let settings = load_yml(opts.actual_opts.input)?;
    info!("Parsed {:?}", settings);
    let scope = opts.actual_opts.scope.clone();
    let json = serde_json::to_value(settings).ax_err_ctx(
        util::formats::ActyxOSCode::ERR_INTERNAL_ERROR,
        "cannot parse provided value",
    )?;
    let mut conn = opts.console_opt.connect().await?;
    match conn
        .request(AdminRequest::SettingsSet {
            scope: scope.clone(),
            json,
            ignore_errors: false,
        })
        .await
    {
        Ok(AdminResponse::SettingsSetResponse(settings)) => Ok(Output {
            scope: super::print_scope(scope),
            settings,
        }),
        Ok(r) => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
        Err(err) => Err(err),
    }
}
