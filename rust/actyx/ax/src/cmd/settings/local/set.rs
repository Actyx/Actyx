use crate::cmd::{
    determine_ax_default_data_dir,
    settings::set::{extract_set_settings_command, Output, SetSettingsCommand},
    AxCliCommand,
};
use ax_core::util::formats::{ActyxOSError, ActyxOSResult};
use futures::{stream, Stream};
use std::path::PathBuf;

use super::lock_and_load_repo;

#[derive(clap::Parser, Debug, Clone)]
pub struct SettingsLocalSetOpts {
    #[command(flatten)]
    actual_opts: SetSettingsCommand,
    /// Path where to store all the data of the Actyx node.
    #[arg(
        long,
        env = "ACTYX_PATH",
        long_help = "Path where to store all the data of the Actyx node. \
          Defaults to creating <current working dir>/ax-data"
    )]
    working_dir: Option<PathBuf>,
}

pub struct SettingsLocalSet;
impl AxCliCommand for SettingsLocalSet {
    type Opt = SettingsLocalSetOpts;
    type Output = Output;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts));
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
            .unwrap_or_else(|e| format!("Unknown error converting settings to yaml: {}", e))
    }
}

pub(crate) async fn run(
    SettingsLocalSetOpts {
        actual_opts,
        working_dir,
    }: SettingsLocalSetOpts,
) -> ActyxOSResult<Output> {
    let working_dir = working_dir
        .map_or_else(determine_ax_default_data_dir, Ok)
        .map_err(|e| ActyxOSError::internal(format!("{}", e)))?;

    let (_lock, repository) = lock_and_load_repo(&working_dir)?;

    let (scope, json) = extract_set_settings_command(actual_opts)?;

    let update = repository.update_settings(&scope, json, false)?;

    Ok(Output {
        scope: scope.to_string(),
        settings: update,
    })
}
