use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::language::Query;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Installs a standing [`Query`] for setting the set of pinned files. The result of this query
/// must evaluate to a single array of hashes. This set of hashes is pinned on the local node for
/// the given [`Duration`].
/// ```
/// use actyx_sdk::service::PrefetchRequest;
/// use std::time::Duration;
///
/// let now = chrono::Utc::now();
/// let query = format!(
///        r#"
/// FEATURES(zøg aggregate timeRange)
/// FROM isLocal &
///      appId(com.actyx) &
///      'files:created' &
///      from({})
/// AGGREGATE ARRAY(_.cid)"#,
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
    /// AQL Query. Must evaluate to a single array of hashes.
    pub query: Query,
    /// How long the files should be pinned (until = now + duration)
    pub duration: Duration,
}
