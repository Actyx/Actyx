use actyx_sdk::AppId;
use certs::{AppDomain, DeveloperCertificate, SignedAppLicense};
use chrono::{DateTime, Utc};
use crypto::PrivateKey;
use lazy_static::lazy_static;
use regex::{Captures, Regex};
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

    /// An expiration date time in ISO 8601 (i.e. 2014-11-28T12:00:09Z).
    /// Takes precedences over `--expires-in`.
    #[structopt(long)]
    expires_at: Option<DateTime<Utc>>,

    /// The amount of time in which the license should expire. The accepted format is
    /// composed of a number followed by a unit (i.e. "10Y"), whitespace is supported
    /// before, between and after the number and the unit. Values are accepted in
    /// descending order according to the size of the unit, the unit character and
    /// ordering is as follows:
    ///
    /// [Y]ear, [M]onth, [w]eek, [d]ay, [h]our, [d]ay, [m]inute, [s]econd
    ///
    /// Note: Years are considered to be 365 days long and months to be 30 days long.
    ///
    /// For example: "1Y 3M 4h" means that the license should expire in 1 year, 3 months and 4 hours.
    #[structopt(long, parse(try_from_str = parse_expires_in))]
    expires_in: Option<DateTime<Utc>>,

    /// Requester's email address
    #[structopt(long)]
    email: String,
}

/// Parsing function for the `expires_in` variable.
/// Accepts strings matching the regex:
/// `^\s*([0-9]+Y)?\s*([0-9]+Y)?\s*([0-9]+Y)?\s*([0-9]+Y)?\s*([0-9]+Y)?\s*([0-9]+Y)?\s*([0-9]+Y)?\s*$`
///
/// Y - year(s) - counted as 365 days
/// M - month(s) - counted as 30 days
/// w - week(s)
/// d - day(s)
/// h - hour(s)
/// m - minute(s)
/// s - second(s)
fn parse_expires_in(expires_in: &str) -> Result<DateTime<Utc>, anyhow::Error> {
    parse_expires_in_as_duration(expires_in).map(|d| Utc::now() + d)
}

// This function is much easier to test for correctness
fn parse_expires_in_as_duration(expires_in: &str) -> Result<chrono::Duration, anyhow::Error> {
    lazy_static! {
        // Named matches for easy extraction, string concat for readability
        static ref RE: Regex = Regex::new(concat!(
            r"^\s*((?P<years>[0-9]+)Y)?\s*",
            r"((?P<months>[0-9]+)M)?\s*",
            r"((?P<weeks>[0-9]+)w)?\s*",
            r"((?P<days>[0-9]+)d)?\s*",
            r"((?P<hours>[0-9]+)h)?\s*",
            r"((?P<minutes>[0-9]+)m)?\s*",
            r"((?P<seconds>[0-9]+)s)?\s*$",
        ))
        .unwrap();
    }

    let captures = RE
        .captures(expires_in)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse string."))?;

    let mut duration = chrono::Duration::zero();
    duration = duration + chrono::Duration::days(365 * get_i64_from_capture(&captures, "years"));
    duration = duration + chrono::Duration::days(30 * get_i64_from_capture(&captures, "months"));
    duration = duration + chrono::Duration::days(7 * get_i64_from_capture(&captures, "weeks"));
    duration = duration + chrono::Duration::days(get_i64_from_capture(&captures, "days"));
    duration = duration + chrono::Duration::hours(get_i64_from_capture(&captures, "hours"));
    duration = duration + chrono::Duration::minutes(get_i64_from_capture(&captures, "minutes"));
    duration = duration + chrono::Duration::seconds(get_i64_from_capture(&captures, "seconds"));

    if duration.is_zero() {
        return Err(anyhow::anyhow!("Expiration interval must be bigger than zero"));
    }
    Ok(duration)
}

// This could've been an extension using a trait, but we're not building a library.
/// Get an i64 from a named capture group. If the group does not exist or parsing
/// the captured value as i64 fails, returns `0` as a default value.
fn get_i64_from_capture(captures: &Captures, name: &str) -> i64 {
    captures
        .name(name)
        .and_then(|m| m.as_str().parse::<i64>().ok())
        .unwrap_or(0)
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
    let expiration_date = opts.expires_at.or(opts.expires_in).ok_or(anyhow::anyhow!(
        "An expiration date must be specified. Use `--expires-at` or `--expires-in`."
    ))?;

    let license = SignedAppLicense::new(opts.actyx_private_key, opts.email, opts.app_id, expiration_date, None)?;
    let serialized = license.to_base64().unwrap();
    println!("{}", serialized);

    Ok(())
}

#[cfg(test)]
mod test_expires_in {
    use crate::parse_expires_in_as_duration;

    // NOTE(duarte): Quickcheck would probably be amazing to test this but I don't have the time to learn it now
    #[test]
    fn test_years() {
        let expected = chrono::Duration::days(365 * 10);
        let result = parse_expires_in_as_duration("10Y").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration("10Y ").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10Y").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10Y ").unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_months() {
        let expected = chrono::Duration::days(30 * 10);
        let result = parse_expires_in_as_duration("10M").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration("10M ").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10M").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10M ").unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_weeks() {
        let expected = chrono::Duration::days(7 * 10);
        let result = parse_expires_in_as_duration("10w").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration("10w ").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10w").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10w ").unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_days() {
        let expected = chrono::Duration::days(10);
        let result = parse_expires_in_as_duration("10d").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration("10d ").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10d").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10d ").unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_hours() {
        let expected = chrono::Duration::hours(10);
        let result = parse_expires_in_as_duration("10h").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration("10h ").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10h").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10h ").unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_minutes() {
        let expected = chrono::Duration::minutes(10);
        let result = parse_expires_in_as_duration("10m").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration("10m ").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10m").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10m ").unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_seconds() {
        let expected = chrono::Duration::seconds(10);
        let result = parse_expires_in_as_duration("10s").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration("10s ").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10s").unwrap();
        assert_eq!(expected, result);
        let result = parse_expires_in_as_duration(" 10s ").unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn test_full() {
        // 2 years, 3 weeks, 21 minutes and 10 seconds
        let expected =
            chrono::Duration::days((365 * 2) + (7 * 3)) + chrono::Duration::minutes(21) + chrono::Duration::seconds(10);

        // Spaces between the number and the unit are not allowed
        let result = parse_expires_in_as_duration("2Y    3 w   21     m 10 s");
        assert!(result.is_err());
        let result = parse_expires_in_as_duration("  2Y    3w   21m 10s  ").unwrap();
        assert_eq!(expected, result);
    }
}
