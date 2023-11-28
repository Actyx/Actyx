mod license;
mod sign;

use crate::cmd::AxCliCommand;
use futures::Future;

use license::LicenseOpts;
use sign::SignOpts;

#[derive(clap::Subcommand, Clone, Debug)]
/// manage app manifests
pub enum AppsOpts {
    /// Create app or node license
    License(LicenseOpts),
    /// Sign application manifest
    Sign(SignOpts),
}

pub fn run(opts: AppsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        AppsOpts::Sign(opt) => sign::AppsSign::output(opt, json),
        AppsOpts::License(opt) => license::AppsLicense::output(opt, json),
    }
}
