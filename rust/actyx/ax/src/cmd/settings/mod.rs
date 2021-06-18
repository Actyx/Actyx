mod get;
mod schema;
mod set;
mod unset;

use crate::cmd::AxCliCommand;
use futures::Future;
use get::GetOpt;
use schema::SchemaOpt;
use set::SetOpt;
use settings::{Scope, ScopeError};
use std::{convert::TryFrom, str::FromStr};
use structopt::StructOpt;
use unset::UnsetOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// manage node settings
pub enum SettingsOpts {
    #[structopt(no_version)]
    /// Configure settings of a node
    Set(SetOpt),
    #[structopt(no_version)]
    /// Remove settings from a node
    Unset(UnsetOpt),
    #[structopt(no_version)]
    /// Get settings from a node
    Get(GetOpt),
    #[structopt(no_version)]
    /// Get setting schemas from a node
    Schema(SchemaOpt),
}

pub fn run(opts: SettingsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        SettingsOpts::Set(opt) => set::SettingsSet::output(opt, json),
        SettingsOpts::Get(opt) => get::SettingsGet::output(opt, json),
        SettingsOpts::Schema(opt) => schema::SettingsSchema::output(opt, json),
        SettingsOpts::Unset(opt) => unset::SettingsUnset::output(opt, json),
    }
}

fn parse_scope(value: &str) -> Result<Scope, ScopeError> {
    if !value.starts_with('/') {
        return Err(ScopeError::MalformedScope(value.to_string()));
    }
    if value == "/" {
        Scope::from_str("com.actyx")
    } else {
        Scope::try_from(format!("com.actyx{}", value))
    }
}

fn print_scope(scope: Scope) -> String {
    let scope = scope.drop_first();
    if scope.is_root() {
        // printing <ROOT> incorrectly as `/`, but this can’t ever happen anyway due to the above function
        "/".to_string()
    } else {
        format!("/{}", scope)
    }
}
