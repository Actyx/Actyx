use crate::cmd::{determine_ax_default_data_dir, settings::get::GetSettingsCommand, AxCliCommand};
use ax_core::util::formats::{ActyxOSError, ActyxOSResult};
use futures::{stream, Stream};
use std::path::PathBuf;

use super::lock_and_load_repo;

#[derive(clap::Parser, Debug, Clone)]
pub struct SettingsLocalGetOpts {
    #[command(flatten)]
    actual_opts: GetSettingsCommand,
    /// Path where to store all the data of the Actyx node.
    #[arg(
        long,
        env = "ACTYX_PATH",
        long_help = "Path where to store all the data of the Actyx node. \
          Defaults to creating <current working dir>/ax-data"
    )]
    working_dir: Option<PathBuf>,
}

pub struct SettingsLocalGet;
impl AxCliCommand for SettingsLocalGet {
    type Opt = SettingsLocalGetOpts;
    type Output = serde_json::Value;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts));
        Box::new(stream::once(r))
    }
    fn pretty(result: Self::Output) -> String {
        serde_yaml::to_string(&result).unwrap_or_else(|e| format!("Unknown error converting settings to yaml: {}", e))
    }
}

pub async fn run(
    SettingsLocalGetOpts {
        actual_opts: GetSettingsCommand { no_defaults, scope },
        working_dir,
    }: SettingsLocalGetOpts,
) -> ActyxOSResult<serde_json::Value> {
    let working_dir = working_dir
        .map_or_else(determine_ax_default_data_dir, Ok)
        .map_err(|e| ActyxOSError::internal(format!("{}", e)))?;

    let (_lock, repository) = lock_and_load_repo(&working_dir)?;

    let res = repository.get_settings(&scope, no_defaults)?;

    Ok(res)
}
