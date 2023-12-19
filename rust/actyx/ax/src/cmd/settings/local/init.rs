use crate::cmd::{determine_ax_default_data_dir, AxCliCommand};
use ax_core::{
    node::{self, lock_working_dir},
    settings::Database,
    util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult},
};
use futures::{stream, Stream};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct SettingLocalInitResult {
    created: bool,
    working_dir: PathBuf,
}

#[derive(clap::Parser, Debug, Clone)]
pub struct SettingsLocalInitOpts {
    /// Enable to fail the command when the assigned WORKING_DIR already has
    /// settings.db initialized within
    #[arg(long)]
    fail_on_existing: bool,
    /// Path where to store all the data of the Actyx node.
    #[arg(
        long,
        env = "ACTYX_PATH",
        long_help = "Path where to store all the data of the Actyx node. \
          Defaults to creating <current working dir>/ax-data"
    )]
    working_dir: Option<PathBuf>,
}

pub struct SettingsLocalInit;
impl AxCliCommand for SettingsLocalInit {
    type Opt = SettingsLocalInitOpts;
    type Output = SettingLocalInitResult;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts));
        Box::new(stream::once(r))
    }
    fn pretty(result: Self::Output) -> String {
        if result.created {
            format!("new actyx settings is created in {}", result.working_dir.display())
        } else {
            format!(
                "actyx settings exists in {}\n nothing is changed.",
                result.working_dir.display()
            )
        }
    }
}

pub async fn run(
    SettingsLocalInitOpts {
        fail_on_existing,
        working_dir,
    }: SettingsLocalInitOpts,
) -> ActyxOSResult<SettingLocalInitResult> {
    let working_dir = working_dir
        .map_or_else(determine_ax_default_data_dir, Ok)
        .map_err(|e| ActyxOSError::internal(format!("{}", e)))?;

    if Database::exists(&working_dir) {
        if fail_on_existing {
            return Err(ActyxOSError::new(
                ActyxOSCode::ERR_INVALID_INPUT,
                format!("database already initialized in {}", working_dir.display()),
            ));
        }

        return Ok(SettingLocalInitResult {
            created: false,
            working_dir,
        });
    }

    std::fs::create_dir_all(working_dir.clone()).map_err(|e| {
        ActyxOSError::new(
            ActyxOSCode::ERR_IO,
            format!("failed creating working directory {}. {:?}", working_dir.display(), e),
        )
    })?;

    let _lock = lock_working_dir(&working_dir).map_err(|e| ActyxOSError::new(ActyxOSCode::ERR_IO, format!("{}", e)))?;

    let _ = node::initialize_repository(&working_dir).map_err(|e| ActyxOSError::internal(format!("{}", e)))?;

    Ok(SettingLocalInitResult {
        created: true,
        working_dir,
    })
}
