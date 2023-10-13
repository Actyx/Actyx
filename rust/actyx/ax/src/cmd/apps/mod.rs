mod license;
mod sign;

use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

use license::LicenseOpts;
pub use sign::{create_signed_app_manifest, SignOpts};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// manage app manifests
pub enum AppsOpts {
    #[structopt(no_version)]
    /// Create app or node license
    License(LicenseOpts),
    #[structopt(no_version)]
    /// Sign application manifest
    Sign(SignOpts),
}

pub fn run(opts: AppsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        AppsOpts::Sign(opt) => sign::AppsSign::output(opt, json),
        AppsOpts::License(opt) => license::AppsLicense::output(opt, json),
    }
}
