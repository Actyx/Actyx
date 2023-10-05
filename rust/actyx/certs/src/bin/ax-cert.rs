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
    expires_at: Option<DateTime<Utc>>,

    #[structopt(long, parse(try_from_str = parse_expires_in))]
    expires_in: Option<DateTime<Utc>>,

    /// Requester's email address
    #[structopt(long)]
    email: String,
}

/// Parsing function for the `expires_in` variable.
/// Accepts strings matching the regex: ^([0-9]+(Y|M|w|d|h|m|s))+$
///
/// Y - year(s) - counted as 365 days
/// M - month(s) - counted as 30 days
/// w - week(s)
/// d - day(s)
/// h - hour(s)
/// m - minute(s)
/// s - second(s)
fn parse_expires_in(expires_in: &str) -> Result<DateTime<Utc>, anyhow::Error> {
    let mut needle = 0;
    let mut duration = chrono::Duration::zero();

    if let Some(i) = expires_in.find('Y') {
        let years: i64 = expires_in[needle..i].trim().parse()?;
        duration = duration + chrono::Duration::days(365 * years);
        needle = i;
    }
    if let Some(i) = expires_in.find('M') {
        let months: i64 = expires_in[needle..i].trim().parse()?;
        duration = duration + chrono::Duration::days(30 * months);
        needle = i;
    }
    if let Some(i) = expires_in.find('w') {
        let weeks: i64 = expires_in[needle..i].trim().parse()?;
        duration = duration + chrono::Duration::days(30 * weeks);
        needle = i;
    }
    if let Some(i) = expires_in.find('d') {
        let days: i64 = expires_in[needle..i].trim().parse()?;
        duration = duration + chrono::Duration::days(30 * days);
        needle = i;
    }
    if let Some(i) = expires_in.find('h') {
        let hours: i64 = expires_in[needle..i].trim().parse()?;
        duration = duration + chrono::Duration::days(30 * hours);
        needle = i;
    }
    if let Some(i) = expires_in.find('m') {
        let minutes: i64 = expires_in[needle..i].trim().parse()?;
        duration = duration + chrono::Duration::days(30 * minutes);
        needle = i;
    }
    if let Some(i) = expires_in.find('s') {
        let seconds: i64 = expires_in[needle..i].trim().parse()?;
        duration = duration + chrono::Duration::days(30 * seconds);
        needle = i;
    }

    if needle == 0 {
        return Err(anyhow::anyhow!("Could not parse string"));
    }
    Ok(DateTime::from(Utc::now() + duration))
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
    let expiration_date = opts
        .expires_at
        .or(opts.expires_in)
        .unwrap_or_else(|| DateTime::from(Utc::now() + chrono::Duration::days(1)));

    let license = SignedAppLicense::new(opts.actyx_private_key, opts.email, opts.app_id, expiration_date, None)?;
    let serialized = license.to_base64().unwrap();
    println!("{}", serialized);

    Ok(())
}
