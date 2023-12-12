use crate::cmd::AxCliCommand;
use ax_core::{certs::app_manifest_signer::sign_manifest_from_files, util::formats::ActyxOSResult};
use ax_sdk::types::AppManifest;
use futures::{stream, Stream};
use std::path::PathBuf;

#[derive(clap::Parser, Clone, Debug)]
/// sign an app manifest
pub struct SignOpts {
    /// Path to certificate that shall be used for signing
    pub path_to_certificate: PathBuf,
    /// Path to app manifest that shall be signed
    pub path_to_manifest: PathBuf,
}

async fn run(opts: SignOpts) -> ActyxOSResult<AppManifest> {
    sign_manifest_from_files(opts.path_to_certificate, opts.path_to_manifest)
}

pub struct AppsSign();

impl AxCliCommand for AppsSign {
    type Opt = SignOpts;
    type Output = AppManifest;
    fn run(opts: SignOpts) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts));
        Box::new(stream::once(r))
    }

    fn pretty(_result: Self::Output) -> String {
        "Provided manifest was updated and signed".to_string()
    }
}
