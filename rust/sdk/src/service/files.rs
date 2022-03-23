///! Files API
///!
///! Check the examples for a complete example for adding, listing, and retrieving files.
use std::time::Duration;

pub use libipld::Cid;
use serde::{Deserialize, Deserializer, Serialize};

use crate::language::{Query, StaticQuery};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Installs a standing [`Query`] for setting the set of pinned files. The results of this query
/// must evaluate to a single hash. This collected set of hashes is pinned on the local node for
/// the given [`Duration`].
/// ```
/// use actyx_sdk::service::PrefetchRequest;
/// use std::time::Duration;
///
/// let now = chrono::Utc::now();
/// let query = format!(
///        r#"
/// FEATURES(zoeg aggregate timeRange)
/// FROM isLocal &
///      appId(com.actyx) &
///      'files:created' &
///      from({})
/// SELECT _.cid"#,
///     now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
/// )
/// .parse()
/// .unwrap();
/// let request = PrefetchRequest {
///     query,
///     duration: Duration::from_secs(60 * 60 * 12),
/// };
/// ```
pub struct PrefetchRequest {
    #[serde(deserialize_with = "deser_prefetch")]
    /// AQL Query. Must evaluate to a single array of hashes.
    pub query: Query<'static>,
    /// How long the files should be pinned (until = now + duration)
    pub duration: Duration,
}
fn deser_prefetch<'de, D: Deserializer<'de>>(d: D) -> Result<Query<'static>, D::Error> {
    Ok(StaticQuery::deserialize(d)?.0)
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryChild {
    pub size: u64,
    pub name: String,
    #[serde(with = "serde_str")]
    pub cid: Cid,
}

/// Response to requesting a file.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum FilesGetResponse {
    File {
        name: String,
        bytes: Vec<u8>,
        mime: String,
    },
    Directory {
        name: String,
        #[serde(with = "serde_str")]
        cid: Cid,
        children: Vec<DirectoryChild>,
    },
}

mod serde_str {
    //! Serializes fields annotated with `#[serde(with = "::util::serde_str")]` with their !
    //! `Display` implementation, deserializes fields using `FromStr`.
    use std::fmt::Display;
    use std::str::FromStr;

    use serde::{de, Deserialize, Deserializer, Serializer};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?.parse().map_err(de::Error::custom)
    }
}
