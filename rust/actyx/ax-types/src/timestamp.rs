use cbor_data::cbor_via;
use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use derive_more::{From, Into};
use libipld::DagCbor;
use serde::{Deserialize, Serialize};
use std::{
    convert::{TryFrom, TryInto},
    fmt::{self, Debug, Display, Formatter},
    ops::{Add, Sub},
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

/// Microseconds since the UNIX epoch, without leap seconds and in UTC
///
/// ```
/// use ax_types::Timestamp;
/// use chrono::{DateTime, Utc, TimeZone};
///
/// let timestamp = Timestamp::now();
/// let micros_since_epoch: u64 = timestamp.into();
/// let date_time: DateTime<Utc> = timestamp.try_into().unwrap();
///
/// assert_eq!(timestamp.as_i64() * 1000, date_time.timestamp_nanos_opt().unwrap());
/// assert_eq!(Timestamp::from(date_time), timestamp);
/// ```
#[derive(
    Copy, Clone, Debug, Default, From, Into, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, DagCbor,
)]
#[ipld(repr = "value")]
pub struct Timestamp(pub u64);

cbor_via!(Timestamp => u64: |x| -> x.0, FROM);

impl Timestamp {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub fn now() -> Timestamp {
        let now = SystemTime::now();
        now.try_into().expect("Time went waaaay backwards")
    }
    pub fn as_i64(self) -> i64 {
        self.0 as i64
    }
}

impl TryFrom<SystemTime> for Timestamp {
    type Error = anyhow::Error;
    fn try_from(st: SystemTime) -> Result<Self, Self::Error> {
        let duration = st.duration_since(UNIX_EPOCH)?;
        Ok(Self::new(duration.as_micros() as u64))
    }
}

impl TryFrom<Timestamp> for DateTime<Utc> {
    type Error = anyhow::Error;

    fn try_from(ts: Timestamp) -> Result<Self, Self::Error> {
        TimeZone::timestamp_micros(&Utc, ts.0 as i64)
            .single()
            .ok_or_else(|| anyhow::anyhow!("supplied timestamp {} is out of range for DateTime<Utc>", ts.0))
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        // dt.timestamp_nanos() multiplies by 1e9 which leads to overflows
        let seconds = dt.timestamp() as u64;
        let micros = seconds * 1_000_000 + dt.timestamp_subsec_micros() as u64;
        Self(micros)
    }
}

impl From<DateTime<FixedOffset>> for Timestamp {
    fn from(dt: DateTime<FixedOffset>) -> Self {
        dt.with_timezone(&Utc).into()
    }
}

impl Sub<u64> for Timestamp {
    type Output = Timestamp;
    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0.saturating_sub(rhs))
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = i64;
    fn sub(self, rhs: Timestamp) -> Self::Output {
        self.0.saturating_sub(rhs.0) as i64
    }
}

impl Sub<std::time::Duration> for Timestamp {
    type Output = Timestamp;
    fn sub(self, duration: std::time::Duration) -> Self::Output {
        Self(self.0.saturating_sub(duration.as_micros() as u64))
    }
}

impl Add<u64> for Timestamp {
    type Output = Timestamp;
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0.saturating_add(rhs))
    }
}

impl Add<std::time::Duration> for Timestamp {
    type Output = Timestamp;
    fn add(self, duration: std::time::Duration) -> Self::Output {
        Self(self.0.saturating_add(duration.as_micros() as u64))
    }
}

impl FromStr for Timestamp {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ts = iso8601_timestamp::Timestamp::parse(s).ok_or(anyhow::anyhow!("failed to parse timestamp"))?;

        Ok(Self(u64::try_from(
            ts.duration_since(iso8601_timestamp::Timestamp::UNIX_EPOCH)
                .whole_microseconds(),
        )?))
    }
}

#[cfg(any(test, feature = "arb"))]
impl quickcheck::Arbitrary for Timestamp {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Timestamp::new(u64::arbitrary(g) & ((2 << 53) - 1))
    }
}

/// A logical timestamp taken from a [`Lamport clock`](https://en.wikipedia.org/wiki/Lamport_timestamps)
///
/// The lamport clock in an Actyx system is increased by the Actyx node whenever:
///
/// - an event is emitted
/// - a heartbeat is received
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default, From, Into, DagCbor,
)]
#[ipld(repr = "value")]
pub struct LamportTimestamp(u64);

cbor_via!(LamportTimestamp => u64);

impl LamportTimestamp {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub fn incr(self) -> Self {
        LamportTimestamp::new(self.0 + 1)
    }
    #[deprecated(since = "0.2.1", note = "use .into()")]
    pub fn as_u64(self) -> u64 {
        self.0
    }
    pub fn as_i64(self) -> i64 {
        self.0 as i64
    }
}

impl From<&LamportTimestamp> for u64 {
    fn from(lt: &LamportTimestamp) -> Self {
        lt.0
    }
}

impl Display for LamportTimestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "LT({})", Into::<u64>::into(*self))
    }
}

impl Add<u64> for LamportTimestamp {
    type Output = LamportTimestamp;
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0.saturating_add(rhs))
    }
}

#[cfg(any(test, feature = "arb"))]
impl quickcheck::Arbitrary for LamportTimestamp {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        LamportTimestamp::new(u64::arbitrary(g))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use libipld::{cbor::DagCborCodec, codec::assert_roundtrip, ipld};
    use std::time::Duration;

    #[test]
    fn timestamp_add_u64() {
        assert_eq!(LamportTimestamp(3) + 3u64, LamportTimestamp(6));
        assert_eq!(LamportTimestamp(u64::MAX) + 3u64, LamportTimestamp(u64::MAX));
    }

    #[test]
    fn lamport_timestamp_add_u64() {
        assert_eq!(Timestamp(3) + 3u64, Timestamp(6));
        assert_eq!(Timestamp(u64::MAX) + 3u64, Timestamp(u64::MAX));
    }

    #[test]
    fn lamport_timestamp_add_duration() {
        assert_eq!(Timestamp(3) + Duration::from_micros(3), Timestamp(6));
        assert_eq!(Timestamp(u64::MAX) + Duration::from_micros(3), Timestamp(u64::MAX));
    }

    #[test]
    fn lamport_timestamp_sub_duration() {
        assert_eq!(Timestamp(30) - Duration::from_micros(3), Timestamp(27));
        assert_eq!(Timestamp(30) - Duration::from_micros(300), Timestamp(u64::MIN));
    }

    #[test]
    fn lamport_timestamp_sub_u64() {
        assert_eq!(Timestamp(30) - 3u64, Timestamp(27));
        assert_eq!(Timestamp(30) - 300u64, Timestamp(u64::MIN));
    }

    #[test]
    fn lamport_timestamp_sub_timestamp() {
        assert_eq!(Timestamp(30) - Timestamp(3), 27);
        assert_eq!(Timestamp(30) - Timestamp(300), 0);
    }

    #[test]
    fn timestamp_libipld() {
        assert_roundtrip(DagCborCodec, &Timestamp::from(0), &ipld!(0));
        assert_roundtrip(DagCborCodec, &Timestamp::from(1), &ipld!(1));
    }

    #[test]
    fn lamport_timestamp_libipld() {
        assert_roundtrip(DagCborCodec, &LamportTimestamp::from(0), &ipld!(0));
        assert_roundtrip(DagCborCodec, &LamportTimestamp::from(1), &ipld!(1));
    }
}
