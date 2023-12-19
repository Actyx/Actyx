mod get;
mod init;
mod set;
mod unset;

use ax_core::{
    node::{self, lock_working_dir},
    settings::Repository,
    util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult},
};

use self::{
    get::{SettingsLocalGet, SettingsLocalGetOpts},
    init::{SettingsLocalInit, SettingsLocalInitOpts},
    set::{SettingsLocalSet, SettingsLocalSetOpts},
    unset::{SettingsLocalUnset, SettingsLocalUnsetOpts},
};
use crate::cmd::AxCliCommand;
use core::future::Future;
use std::path::Path;

#[derive(clap::Subcommand, Debug, Clone)]
pub enum SettingsLocalOpts {
    Get(SettingsLocalGetOpts),
    Init(SettingsLocalInitOpts),
    Set(SettingsLocalSetOpts),
    Unset(SettingsLocalUnsetOpts),
}

pub fn run(arg: SettingsLocalOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match arg {
        SettingsLocalOpts::Get(x) => SettingsLocalGet::output(x, json),
        SettingsLocalOpts::Init(x) => SettingsLocalInit::output(x, json),
        SettingsLocalOpts::Set(x) => SettingsLocalSet::output(x, json),
        SettingsLocalOpts::Unset(x) => SettingsLocalUnset::output(x, json),
    }
}

pub(crate) fn lock_and_load_repo(working_dir: &Path) -> ActyxOSResult<(fslock::LockFile, Repository)> {
    if !working_dir.exists() {
        return Err(ActyxOSError::new(
            ActyxOSCode::ERR_INVALID_INPUT,
            format!("{} does not exist", &working_dir.display()),
        ));
    };

    let lock = lock_working_dir(working_dir).map_err(|e| ActyxOSError::new(ActyxOSCode::ERR_IO, format!("{}", e)))?;

    let repository = node::initialize_repository(working_dir).map_err(|e| ActyxOSError::internal(format!("{}", e)))?;

    Ok((lock, repository))
}
