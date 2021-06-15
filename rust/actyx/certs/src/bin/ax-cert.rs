use actyx_sdk::AppId;
use certs::{AppDomain, DeveloperCertificate, SignedAppLicense};
use chrono::{DateTime, Utc};
use crypto::PrivateKey;
use structopt::StructOpt;
use util::version::NodeVersion;

#[derive(StructOpt, Debug)]
struct DevCertOpts {
    #[structopt(long, env, hide_env_values = true)]
    /// Actyx private key
    actyx_private_key: PrivateKey,

    #[structopt(long, env, hide_env_values = true)]
    /// Developer's private key, if omitted one will be generated
    dev_private_key: Option<PrivateKey>,

    /// Certificate's allowed app domains
    #[structopt(long, required = true)]
    app_domains: Vec<String>,
}

#[derive(StructOpt, Debug)]
struct AppLicenseOpts {
    #[structopt(long, env, hide_env_values = true)]
    /// Actyx private key
    actyx_private_key: PrivateKey,

    /// App id
    #[structopt(long)]
    app_id: AppId,

    /// ISO 8601 (i.e. 2014-11-28T12:00:09Z) expiration date time
    #[structopt(long)]
    expires_at: DateTime<Utc>,

    /// Requester's email address
    #[structopt(long)]
    email: String,
}

#[derive(StructOpt, Debug)]
enum Commands {
    /// Creates developer certificate
    DevCert(DevCertOpts),
    /// Creates app license
    AppLicense(AppLicenseOpts),
}

#[derive(StructOpt, Debug)]
#[structopt(
    name = "Actyx developer certificate utility",
    about = "Manages Actyx developer certificates",
    rename_all = "kebab-case"
)]
struct Opts {
    #[structopt(subcommand)]
    commands: Option<Commands>,
    #[structopt(long)]
    version: bool,
}

fn main() -> anyhow::Result<()> {
    let opts = Opts::from_args();

    match opts {
        Opts { version: true, .. } => {
            let app = Opts::clap();
            let mut buf = Vec::new();
            app.write_version(&mut buf).unwrap();
            let bin_version = std::str::from_utf8(buf.as_slice()).unwrap().to_string();
            println!("{} {}", bin_version, NodeVersion::get());
            Ok(())
        }
        Opts {
            commands: Some(cmd), ..
        } => match cmd {
            Commands::DevCert(opts) => create_dev_cert(opts),
            Commands::AppLicense(opts) => create_app_license(opts),
        },
        _ => {
            let mut app = Opts::clap();
            app.write_long_help(&mut std::io::stderr()).unwrap();
            println!();
            Ok(())
        }
    }
}

fn create_dev_cert(opts: DevCertOpts) -> anyhow::Result<()> {
    let dev_private_key = opts.dev_private_key.unwrap_or_else(PrivateKey::generate);
    let app_domains: Vec<AppDomain> = opts
        .app_domains
        .iter()
        .map(|app_domain| {
            app_domain
                .parse()
                .map_err(|err| anyhow::anyhow!("Failed to parse app domain '{}'. {}", app_domain, err))
        })
        .collect::<anyhow::Result<_>>()?;

    let dev_cert = DeveloperCertificate::new(dev_private_key, app_domains, opts.actyx_private_key)?;
    let serialized = serde_json::to_string(&dev_cert)?;
    println!("{}", serialized);

    Ok(())
}

fn create_app_license(opts: AppLicenseOpts) -> anyhow::Result<()> {
    let license = SignedAppLicense::new(opts.actyx_private_key, opts.email, opts.app_id, opts.expires_at, None)?;
    let serialized = license.to_base64().unwrap();
    println!("{}", serialized);

    Ok(())
}
