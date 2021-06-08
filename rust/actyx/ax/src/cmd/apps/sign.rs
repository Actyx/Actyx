use crate::{
    cmd::{AxCliCommand, KeyPathWrapper},
    private_key::AxPrivateKey,
};
use actyx_sdk::AppManifest;
use certs::{DeveloperCertificate, SignedAppManifest};
use futures::{stream, Stream};
use std::{convert::TryInto, fs, path::PathBuf};
use structopt::StructOpt;
use util::formats::{ActyxOSCode, ActyxOSResult, ActyxOSResultExt};

#[derive(StructOpt, Debug)]
pub struct SignOpts {
    #[structopt(short, long, default_value)]
    /// File from which the identity (private key) for app manifest signing is used
    identity: KeyPathWrapper,
    /// Path to certificate that shall be used for signing
    path_to_certificate: PathBuf,
    /// Path to app manifest that shall be signed
    path_to_manifest: PathBuf,
}

async fn run(opts: SignOpts) -> ActyxOSResult<SignedAppManifest> {
    let SignOpts {
        identity,
        path_to_certificate,
        path_to_manifest,
    } = opts;
    let dev_private_key: AxPrivateKey = identity.try_into()?;
    let dev_cert = fs::read_to_string(path_to_certificate)
        .ax_err_ctx(ActyxOSCode::ERR_IO, "Failed to read developer certificate")?;
    let dev_cert: DeveloperCertificate = serde_json::from_str(&dev_cert).ax_err_ctx(
        ActyxOSCode::ERR_INVALID_INPUT,
        "Failed to deserialize developer certificate",
    )?;
    let app_manifest =
        fs::read_to_string(path_to_manifest.clone()).ax_err_ctx(ActyxOSCode::ERR_IO, "Failed to read app manifest")?;
    let app_manifest: AppManifest = serde_json::from_str(&app_manifest)
        .ax_err_ctx(ActyxOSCode::ERR_INVALID_INPUT, "Failed to deserialize app manifest")?;

    let signed_manifest = SignedAppManifest::new(
        app_manifest.app_id,
        app_manifest.display_name,
        app_manifest.version,
        dev_private_key.private_key(),
        dev_cert,
    )
    .ax_err_ctx(ActyxOSCode::ERR_INVALID_INPUT, "Failed to create signed manifest")?;
    let serialized = serde_json::to_string(&signed_manifest)
        .ax_err_ctx(ActyxOSCode::ERR_IO, "Failed to serialize signed app manifest")?;
    fs::write(path_to_manifest, serialized).ax_err_ctx(ActyxOSCode::ERR_IO, "Failed to overwrite app manifest")?;

    Ok(signed_manifest)
}

pub struct AppsSign();

impl AxCliCommand for AppsSign {
    type Opt = SignOpts;
    type Output = SignedAppManifest;
    fn run(opts: SignOpts) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts));
        Box::new(stream::once(r))
    }

    fn pretty(_result: Self::Output) -> String {
        "Provided manifest was updated and signed".to_string()
    }
}
