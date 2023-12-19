use ax_core::util::formats::{ActyxOSError, ActyxOSResult};
use futures::{stream, Stream};
use std::path::PathBuf;

use crate::cmd::{
    determine_ax_default_data_dir,
    settings::unset::{Output, UnsetSettingsCommand},
    AxCliCommand,
};

use super::lock_and_load_repo;

#[derive(clap::Parser, Debug, Clone)]
pub struct SettingsLocalUnsetOpts {
    #[command(flatten)]
    actual_opts: UnsetSettingsCommand,
    /// Path where to store all the data of the Actyx node.
    #[arg(
        long,
        env = "ACTYX_PATH",
        long_help = "Path where to store all the data of the Actyx node. \
          Defaults to creating <current working dir>/ax-data"
    )]
    working_dir: Option<PathBuf>,
}

pub struct SettingsLocalUnset;
impl AxCliCommand for SettingsLocalUnset {
    type Opt = SettingsLocalUnsetOpts;
    type Output = Output;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts));
        Box::new(stream::once(r))
    }
    fn pretty(result: Self::Output) -> String {
        format!("Successfully unset settings at {}.", result.scope)
    }
}

pub(crate) async fn run(
    SettingsLocalUnsetOpts {
        actual_opts,
        working_dir,
    }: SettingsLocalUnsetOpts,
) -> ActyxOSResult<Output> {
    let working_dir = working_dir
        .map_or_else(determine_ax_default_data_dir, Ok)
        .map_err(|e| ActyxOSError::internal(format!("{}", e)))?;

    let (_lock, repository) = lock_and_load_repo(&working_dir)?;

    repository.clear_settings(&actual_opts.scope)?;

    Ok(Output {
        scope: actual_opts.scope.to_string(),
    })
}
