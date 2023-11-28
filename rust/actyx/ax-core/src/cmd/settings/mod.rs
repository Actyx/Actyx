mod get;
mod schema;
mod set;
mod unset;

use crate::{
    cmd::AxCliCommand,
    settings::{Scope, ScopeError},
};
use futures::Future;
use get::GetOpt;
use schema::SchemaOpt;
use set::SetOpt;
use std::{convert::TryFrom, str::FromStr};
use unset::UnsetOpt;

#[derive(clap::Subcommand, Debug, Clone)]
/// manage node settings
pub enum SettingsOpts {
    /// Configure settings of a node
    Set(SetOpt),
    /// Remove settings from a node
    Unset(UnsetOpt),
    /// Get settings from a node
    Get(GetOpt),
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
