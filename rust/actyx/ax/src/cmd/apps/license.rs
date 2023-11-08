use crate::{
    certs::SignedAppLicense,
    cmd::AxCliCommand,
    crypto::PrivateKey,
    util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt},
};
use actyx_sdk::AppId;
use chrono::{DateTime, Utc};
use futures::{stream::once, FutureExt, Stream};
use lazy_static::lazy_static;
use regex::Regex;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
pub struct LicenseOpts {
    /// The secret key used to sign the license
    /// (this must match the AX_PUBLIC_KEY your `actyx` binary has been compiled with).
    #[structopt(long, short = "A", env, hide_env_values = true)]
    ax_secret_key: PrivateKey,

    /// The app id of the app to create a license for,
    /// use `com.actyx.node` to create a node license.
    #[structopt(long)]
    app_id: AppId,

    /// An expiration date time in ISO 8601 (i.e. 2014-11-28T12:00:09Z),
    /// takes precedence over `--expires-in`.
    #[structopt(long)]
    expires_at: Option<DateTime<Utc>>,

    /// The amount of time in which the license should expire.
    ///
    /// The accepted format is
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
    #[structopt(long, short = "e", parse(try_from_str = parse_expires_in))]
    expires_in: Option<DateTime<Utc>>,

    /// Requester's email address
    #[structopt(long)]
    email: String,
}

pub struct AppsLicense;

impl AxCliCommand for AppsLicense {
    type Opt = LicenseOpts;
    type Output = String;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(once(
            async move {
                let expiration_date = opts.expires_at.or(opts.expires_in).ok_or(ActyxOSError::new(
                    ActyxOSCode::ERR_INVALID_INPUT,
                    "An expiration date must be specified. Use `--expires-at` or `--expires-in`.",
                ))?;

                let license = SignedAppLicense::new(opts.ax_secret_key, opts.email, opts.app_id, expiration_date, None)
                    .ax_err(ActyxOSCode::ERR_INTERNAL_ERROR)?;
                license.to_base64().ax_err(ActyxOSCode::ERR_INTERNAL_ERROR)
            }
            .boxed(),
        ))
    }

    fn pretty(result: Self::Output) -> String {
        result
    }
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

    let mut add_from_captures = |name: &str, factor: i64, unit: fn(i64) -> chrono::Duration| {
        let quantity = captures
            .name(name)
            .and_then(|m| m.as_str().parse::<i64>().ok())
            .unwrap_or(0);
        duration = duration + unit(quantity * factor);
    };

    add_from_captures("years", 365, chrono::Duration::days);
    add_from_captures("months", 30, chrono::Duration::days);
    add_from_captures("weeks", 7, chrono::Duration::days);
    add_from_captures("days", 1, chrono::Duration::days);
    add_from_captures("hours", 1, chrono::Duration::hours);
    add_from_captures("minutes", 1, chrono::Duration::minutes);
    add_from_captures("seconds", 1, chrono::Duration::seconds);

    if duration.is_zero() {
        return Err(anyhow::anyhow!("Expiration interval must be bigger than zero"));
    }
    Ok(duration)
}

#[cfg(test)]
mod test_expires_in {
    use super::parse_expires_in_as_duration;

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
