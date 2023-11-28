mod license;
mod sign;

use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

use license::LicenseOpts;
pub use sign::SignOpts;

#[derive(StructOpt, Debug)]
#[structopt(version = ax_core::util::version::VERSION.as_str())]
/// manage app manifests
pub enum AppsOpts {
    /// Create app or node license
    #[structopt(no_version)]
    License(LicenseOpts),
    /// Sign application manifest
    #[structopt(no_version)]
    Sign(SignOpts),
}

pub fn run(opts: AppsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        AppsOpts::Sign(opt) => sign::AppsSign::output(opt, json),
        AppsOpts::License(opt) => license::AppsLicense::output(opt, json),
    }
}
