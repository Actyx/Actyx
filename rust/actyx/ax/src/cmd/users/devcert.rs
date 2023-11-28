use crate::cmd::AxCliCommand;
use ax_core::{
    certs::{AppDomain, DeveloperCertificateInput, ManifestDeveloperCertificate},
    crypto::{PrivateKey, PublicKey},
    util::formats::{ActyxOSCode, ActyxOSResult, ActyxOSResultExt},
};
use futures::{stream::once, FutureExt, Stream};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = ax_core::util::version::VERSION.as_str())]
pub struct DevCertOpts {
    /// The secret key used to sign the certificate
    /// (this must match the AX_PUBLIC_KEY your `actyx` binary has been compiled with).
    #[structopt(long, short = "A", env, hide_env_values = true)]
    ax_secret_key: PrivateKey,

    /// The developer's public key.
    #[structopt(long, short)]
    dev_public_key: PublicKey,

    /// The app id domains for which to certify the developer.
    #[structopt(long, short)]
    app_domains: Vec<AppDomain>,
}

pub struct UsersDevCert;

impl AxCliCommand for UsersDevCert {
    type Opt = DevCertOpts;
    type Output = String;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(once(
            async move {
                let input = DeveloperCertificateInput::new(opts.dev_public_key, opts.app_domains);
                let dev_cert = ManifestDeveloperCertificate::new(input, opts.ax_secret_key)
                    .ax_err(ActyxOSCode::ERR_INTERNAL_ERROR)?;
                serde_json::to_string(&dev_cert).ax_err(ActyxOSCode::ERR_INTERNAL_ERROR)
            }
            .boxed(),
        ))
    }

    fn pretty(result: Self::Output) -> String {
        result
    }
}
