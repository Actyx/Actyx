mod get;
mod schema;
mod scopes;
mod set;
mod unset;

use crate::cmd::AxCliCommand;
use futures::Future;
use get::GetOpt;
use schema::SchemaOpt;
use scopes::ScopesOpt;
use set::SetOpt;
use structopt::StructOpt;
use unset::UnsetOpt;

#[derive(StructOpt, Debug)]
/// Manage node or app settings
pub enum SettingsOpts {
    #[structopt(name = "set")]
    /// Configure settings of a node
    Set(SetOpt),
    #[structopt(name = "unset")]
    /// Remove settings from a node
    Unset(UnsetOpt),
    #[structopt(name = "get")]
    /// Get settings from a node
    Get(GetOpt),
    #[structopt(name = "scopes")]
    /// Get setting scopes from a node
    Scopes(ScopesOpt),
    #[structopt(name = "schema")]
    /// Get setting schemas from a node
    Schema(SchemaOpt),
}

pub fn run(opts: SettingsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        SettingsOpts::Set(opt) => set::SettingsSet::output(opt, json),
        SettingsOpts::Get(opt) => get::SettingsGet::output(opt, json),
        SettingsOpts::Scopes(opt) => scopes::SettingsScopes::output(opt, json),
        SettingsOpts::Schema(opt) => schema::SettingsSchema::output(opt, json),
        SettingsOpts::Unset(opt) => unset::SettingsUnset::output(opt, json),
    }
}
