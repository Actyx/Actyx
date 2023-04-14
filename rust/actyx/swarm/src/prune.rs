use crate::{streams::OwnStreamGuard, BanyanStore, EphemeralEventsConfig, Link};
use actyx_sdk::{Payload, Timestamp};
use anyhow::Context;
use banyan::Tree;
use futures::future::{join_all, FutureExt};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{de::Visitor, Deserialize, Serialize};
use std::{
    convert::TryInto,
    future,
    str::FromStr,
    time::{Duration, SystemTime},
};
use trees::{
    axtrees::AxTrees,
    query::{OffsetQuery, TimeQuery},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StreamSize {
    Bytes(u64),
    KiloBytes(u64),
    MegaBytes(u64),
    GigaBytes(u64),
    KibiBytes(u64),
    MebiBytes(u64),
    GibiBytes(u64),
}

impl Serialize for StreamSize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            StreamSize::Bytes(value) => serializer.serialize_str(&format!("{}B", value)),
            StreamSize::KiloBytes(value) => serializer.serialize_str(&format!("{}kB", value)),
            StreamSize::MegaBytes(value) => serializer.serialize_str(&format!("{}MB", value)),
            StreamSize::GigaBytes(value) => serializer.serialize_str(&format!("{}GB", value)),
            StreamSize::KibiBytes(value) => serializer.serialize_str(&format!("{}KiB", value)),
            StreamSize::MebiBytes(value) => serializer.serialize_str(&format!("{}MiB", value)),
            StreamSize::GibiBytes(value) => serializer.serialize_str(&format!("{}GiB", value)),
        }
    }
}

struct StreamSizeVisitor;

impl Visitor<'_> for StreamSizeVisitor {
    type Value = StreamSize;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a string containing a non-zero positive number, suffixed by one of the following: B, kB, MB, GB, KiB, MiB, GiB")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        StreamSize::from_str(v).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for StreamSize {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(StreamSizeVisitor)
    }
}

impl From<u64> for StreamSize {
    /// Assumes that the value is in bytes.
    fn from(b: u64) -> Self {
        StreamSize::Bytes(b)
    }
}

impl From<StreamSize> for u64 {
    fn from(stream_size: StreamSize) -> Self {
        match stream_size {
            StreamSize::Bytes(v) => v,
            StreamSize::KiloBytes(v) => v * 1000,
            StreamSize::MegaBytes(v) => v * 1000 * 1000,
            StreamSize::GigaBytes(v) => v * 1000 * 1000 * 1000,
            StreamSize::KibiBytes(v) => v * 1024,
            StreamSize::MebiBytes(v) => v * 1024 * 1024,
            StreamSize::GibiBytes(v) => v * 1024 * 1024 * 1024,
        }
    }
}

impl FromStr for StreamSize {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^([1-9][0-9]*)(B|kB|MB|GB|KiB|MiB|GiB)$").unwrap();
        }
        let captures = RE
            .captures(s)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse string."))?;
        let value = captures.get(1).map(|v| v.as_str()).unwrap_or("0").parse::<u64>()?;
        let unit = captures.get(2).map(|u| u.as_str());
        Ok(match unit {
            None | Some("B") => Self::Bytes(value),
            Some("kB") => Self::KiloBytes(value),
            Some("MB") => Self::MegaBytes(value),
            Some("GB") => Self::GigaBytes(value),
            Some("KiB") => Self::KibiBytes(value),
            Some("MiB") => Self::MebiBytes(value),
            Some("GiB") => Self::GibiBytes(value),
            _ => unreachable!("This should've been covered by the regex."),
        })
    }
}

#[cfg(test)]
mod test_stream_size {
    use std::str::FromStr;

    use crate::prune::StreamSize;

    #[test]
    fn test_from_kb() {
        assert_eq!(StreamSize::from_str("1kB").unwrap(), StreamSize::KiloBytes(1));
        assert_eq!(StreamSize::from_str("1190kB").unwrap(), StreamSize::KiloBytes(1190));
        assert_eq!(
            StreamSize::from_str("9340123kB").unwrap(),
            StreamSize::KiloBytes(9340123)
        );
    }

    #[test]
    fn test_from_mb() {
        assert_eq!(StreamSize::from_str("1MB").unwrap(), StreamSize::MegaBytes(1));
        assert_eq!(StreamSize::from_str("1190MB").unwrap(), StreamSize::MegaBytes(1190));
        assert_eq!(
            StreamSize::from_str("9340123MB").unwrap(),
            StreamSize::MegaBytes(9340123)
        );
    }

    #[test]
    fn test_from_gb() {
        assert_eq!(StreamSize::from_str("1GB").unwrap(), StreamSize::GigaBytes(1));
        assert_eq!(StreamSize::from_str("1190GB").unwrap(), StreamSize::GigaBytes(1190));
        assert_eq!(
            StreamSize::from_str("9340123GB").unwrap(),
            StreamSize::GigaBytes(9340123)
        );
    }

    #[test]
    fn test_from_kib() {
        assert_eq!(StreamSize::from_str("1KiB").unwrap(), StreamSize::KibiBytes(1));
        assert_eq!(StreamSize::from_str("1190KiB").unwrap(), StreamSize::KibiBytes(1190));
        assert_eq!(
            StreamSize::from_str("9340123KiB").unwrap(),
            StreamSize::KibiBytes(9340123)
        );
    }

    #[test]
    fn test_from_mib() {
        assert_eq!(StreamSize::from_str("1MiB").unwrap(), StreamSize::MebiBytes(1));
        assert_eq!(StreamSize::from_str("1190MiB").unwrap(), StreamSize::MebiBytes(1190));
        assert_eq!(
            StreamSize::from_str("9340123MiB").unwrap(),
            StreamSize::MebiBytes(9340123)
        );
    }

    #[test]
    fn test_from_gib() {
        assert_eq!(StreamSize::from_str("1GiB").unwrap(), StreamSize::GibiBytes(1));
        assert_eq!(StreamSize::from_str("1190GiB").unwrap(), StreamSize::GibiBytes(1190));
        assert_eq!(
            StreamSize::from_str("9340123GiB").unwrap(),
            StreamSize::GibiBytes(9340123)
        );
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum StreamAge {
    Milliseconds(u64),
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
    Weeks(u64),
}

impl Serialize for StreamAge {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            StreamAge::Milliseconds(value) => serializer.serialize_str(&format!("{}ms", value)),
            StreamAge::Seconds(value) => serializer.serialize_str(&format!("{}s", value)),
            StreamAge::Minutes(value) => serializer.serialize_str(&format!("{}m", value)),
            StreamAge::Hours(value) => serializer.serialize_str(&format!("{}h", value)),
            StreamAge::Days(value) => serializer.serialize_str(&format!("{}d", value)),
            StreamAge::Weeks(value) => serializer.serialize_str(&format!("{}w", value)),
        }
    }
}

struct StreamAgeVisitor;

impl Visitor<'_> for StreamAgeVisitor {
    type Value = StreamAge;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(
            "a string containing a non-zero positive number, suffixed by one of the following: s, m, h, d, w",
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        StreamAge::from_str(v).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for StreamAge {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(StreamAgeVisitor)
    }
}

impl From<StreamAge> for u64 {
    fn from(age: StreamAge) -> Self {
        match age {
            StreamAge::Milliseconds(value) => value,
            StreamAge::Seconds(value) => value * 1000,
            StreamAge::Minutes(value) => value * 1000 * 60,
            StreamAge::Hours(value) => value * 1000 * 60 * 60,
            StreamAge::Days(value) => value * 1000 * 24 * 60 * 60,
            StreamAge::Weeks(value) => value * 1000 * 7 * 24 * 60 * 60,
        }
    }
}

impl From<StreamAge> for Duration {
    fn from(age: StreamAge) -> Self {
        match age {
            StreamAge::Milliseconds(value) => Duration::from_millis(value),
            StreamAge::Seconds(value) => Duration::from_secs(value),
            StreamAge::Minutes(value) => Duration::from_secs(value * 60),
            StreamAge::Hours(value) => Duration::from_secs(value * 60 * 60),
            StreamAge::Days(value) => Duration::from_secs(value * 24 * 60 * 60),
            StreamAge::Weeks(value) => Duration::from_secs(value * 7 * 24 * 60 * 60),
        }
    }
}

impl FromStr for StreamAge {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new("^([1-9][0-9]*)(s|m|h|d|w)$").unwrap();
        }
        let captures = RE
            .captures(s)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse string."))?;
        let value = captures.get(1).map(|v| v.as_str()).unwrap_or("0").parse::<u64>()?;
        let unit = captures.get(2).map(|u| u.as_str()).unwrap_or("s");
        Ok(match unit {
            "s" => Self::Seconds(value),
            "m" => Self::Minutes(value),
            "h" => Self::Hours(value),
            "d" => Self::Days(value),
            "w" => Self::Weeks(value),
            _ => unreachable!("This should've been covered by the regex."),
        })
    }
}

#[cfg(test)]
mod test_stream_age {
    use std::str::FromStr;

    use crate::prune::StreamAge;

    #[test]
    fn test_from_seconds() {
        assert_eq!(StreamAge::from_str("1s").unwrap(), StreamAge::Seconds(1));
        assert_eq!(StreamAge::from_str("100s").unwrap(), StreamAge::Seconds(100));
        assert_eq!(StreamAge::from_str("9010s").unwrap(), StreamAge::Seconds(9010));
    }

    #[test]
    fn test_from_minutes() {
        assert_eq!(StreamAge::from_str("1m").unwrap(), StreamAge::Minutes(1));
        assert_eq!(StreamAge::from_str("100m").unwrap(), StreamAge::Minutes(100));
        assert_eq!(StreamAge::from_str("9010m").unwrap(), StreamAge::Minutes(9010));
    }

    #[test]
    fn test_from_hours() {
        assert_eq!(StreamAge::from_str("1h").unwrap(), StreamAge::Hours(1));
        assert_eq!(StreamAge::from_str("100h").unwrap(), StreamAge::Hours(100));
        assert_eq!(StreamAge::from_str("9010h").unwrap(), StreamAge::Hours(9010));
    }

    #[test]
    fn test_from_days() {
        assert_eq!(StreamAge::from_str("1d").unwrap(), StreamAge::Days(1));
        assert_eq!(StreamAge::from_str("100d").unwrap(), StreamAge::Days(100));
        assert_eq!(StreamAge::from_str("9010d").unwrap(), StreamAge::Days(9010));
    }

    #[test]
    fn test_from_weeks() {
        assert_eq!(StreamAge::from_str("1w").unwrap(), StreamAge::Weeks(1));
        assert_eq!(StreamAge::from_str("100w").unwrap(), StreamAge::Weeks(100));
        assert_eq!(StreamAge::from_str("9010w").unwrap(), StreamAge::Weeks(9010));
    }
}

/// Note: Events are kept on a best-effort basis, potentially violating the
/// constraints expressed by this config.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RetainConfig {
    /// Retains the last `n` events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_events: Option<u64>,
    /// Retain all events between `now - duration` and `now` (in milliseconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<StreamAge>,
    /// Retain the last events up to the provided size in bytes. Note that only
    /// the value bytes are taken into account, no overhead from keys, indexes,
    /// etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<StreamSize>,
}

impl RetainConfig {
    /// Limit the number of events to keep.
    pub fn events(events: u64) -> Self {
        Self {
            max_events: Some(events),
            max_age: None,
            max_size: None,
        }
    }

    pub fn age_from_millis(age: u64) -> Self {
        Self {
            max_events: None,
            max_age: Some(StreamAge::Milliseconds(age)),
            max_size: None,
        }
    }

    pub fn age_from_seconds(age: u64) -> Self {
        Self {
            max_events: None,
            max_age: Some(StreamAge::Seconds(age)),
            max_size: None,
        }
    }

    /// Limit the total size of the events to keep (in bytes).
    pub fn size(size: u64) -> Self {
        Self {
            max_events: None,
            max_age: None,
            max_size: Some(StreamSize::Bytes(size)),
        }
    }
}

fn calculate_emit_from(store: &BanyanStore, tree: Tree<AxTrees, Payload>, size: u64) -> u64 {
    let mut iter = store.data.forest.iter_index_reverse(&tree, banyan::query::AllQuery);
    let mut bytes = 0u64;
    let mut current_offset = tree.count();
    loop {
        if let Some(maybe_index) = iter.next() {
            let index = maybe_index.unwrap();
            // If we want to be a bit smarter here, we need to extend
            // `banyan` for a more elaborated traversal API. For now a plain
            // iterator is enough, and will be for a long time.
            if let banyan::index::Index::Leaf(l) = index {
                // Only the value bytes are taken into account
                bytes += l.value_bytes;
                current_offset -= l.keys().count() as u64;
                if bytes >= size {
                    tracing::debug!(
                        "Hitting size target {} > {}. \
                            Results in min offset (non-inclusive) {}",
                        bytes + l.value_bytes,
                        size,
                        current_offset
                    );
                    break current_offset;
                }
            }
        } else {
            tracing::debug!("No change needed as tree size {} < {}", bytes, size);
            break 0u64;
        }
    }
}

// The timestamp parameter is used has an hack around having to use a fake system clock
// to make testing this function deterministic
// FIXME: use the a clock instead
fn prune_stream(
    store: &BanyanStore,
    mut stream: OwnStreamGuard<'_>,
    config: &RetainConfig,
    timestamp: Timestamp,
) -> anyhow::Result<Option<Link>> {
    let stream_nr = stream.stream_nr();
    store.transform_stream(&mut stream, |transaction, tree| {
        let _ = tracing::debug_span!("prune", stream_nr = u64::from(stream_nr)).entered();
        transaction.pack(tree)?;
        if let Some(age) = config.max_age {
            let emit_after = timestamp - Duration::from(age);
            let query = TimeQuery::from(emit_after..);
            tracing::debug!("Age: events on {}; retain {:?}", stream_nr, query);
            transaction.retain(tree, &query)?;
        }
        if let Some(count) = config.max_events {
            let max = tree.count();
            let lower_bound = max.saturating_sub(count);
            let query = OffsetQuery::from(lower_bound..);
            tracing::debug!("Count: events on {}; retain {:?}", stream_nr, query);
            transaction.retain(tree, &query)?;
        }
        if let Some(size) = config.max_size {
            let emit_from = calculate_emit_from(store, tree.snapshot(), size.into());
            if emit_from > 0 {
                let query = OffsetQuery::from(emit_from..);
                tracing::debug!("Size: events on {}; retain {:?}", stream_nr, query);
                transaction.retain(tree, &query)?;
            }
        }
        Ok(())
    })?;
    Ok(stream.snapshot().link())
}

/// Prunes all ephemeral events for the streams configured via the respective
/// [`RetainConfig`] in [`EphemeralEventsConfig`] in parallel. After all streams
/// have been cleaned, waits for the duration given in
/// [`EphemeralEventsConfig::interval`].
/// Note that any unsealed nodes remain untouched.
pub(crate) async fn prune(store: BanyanStore, config: EphemeralEventsConfig) {
    loop {
        tokio::time::sleep(config.interval).await;
        let tasks = config.streams.iter().map(|(stream_name, cfg)| {
            let store = store.clone();
            tracing::debug!("Checking ephemeral event conditions for {}", stream_name);

            let stream_nr = store.data.routing_table.stream_mapping.get(stream_name).copied();

            let Some(stream_nr) = stream_nr else {
                return future::ready(()).left_future();
            };

            let fut = async move {
                let stream = store.get_or_create_own_stream(stream_nr).unwrap();
                let guard = stream.lock().await;
                prune_stream(&store, guard, cfg, Timestamp::now())
            };

            fut.map(move |res| match res {
                Ok(Some(new_root)) => {
                    tracing::debug!("Ephemeral events on {}: New root {}", stream_nr, new_root);
                }
                Err(e) => {
                    tracing::error!("Error trying to clean ephemeral events in {}: {}", stream_nr, e);
                }
                _ => {}
            })
            .right_future()
        });
        join_all(tasks).await;
    }
}

#[cfg(test)]
mod test {
    use std::{collections::BTreeMap, iter::once, str::FromStr, sync::Arc};

    use actyx_sdk::{app_id, language::TagExpr, tags, AppId, Payload, StreamNr};
    use ax_futures_util::prelude::AxStreamExt;
    use futures::{future, stream, StreamExt, TryStreamExt};

    use super::*;
    use crate::{BanyanConfig, EventRoute, SwarmConfig};
    use acto::ActoRef;
    use itertools::Either;
    use parking_lot::Mutex;

    fn app_id() -> AppId {
        app_id!("test")
    }

    async fn create_store() -> anyhow::Result<BanyanStore> {
        util::setup_logger();
        let cfg: SwarmConfig = SwarmConfig {
            node_name: Some("ephemeral".to_owned()),
            topic: "topic".into(),
            enable_mdns: false,
            listen_addresses: Arc::new(Mutex::new("127.0.0.1:0".parse().unwrap())),
            ephemeral_event_config: EphemeralEventsConfig {
                // no-op config
                interval: Duration::from_secs(300_000_000),
                streams: BTreeMap::default(),
            },
            banyan_config: BanyanConfig {
                tree: banyan::Config::debug(),
                ..Default::default()
            },
            event_routes: vec![EventRoute::new(
                TagExpr::from_str("'test'").unwrap(),
                "test_stream".to_string(),
            )],
            ..SwarmConfig::basic()
        };
        BanyanStore::new(cfg, ActoRef::blackhole()).await
    }

    async fn publish_events(event_count: u64) -> anyhow::Result<BanyanStore> {
        let store = create_store().await?;
        let events = (0..event_count)
            .into_iter()
            .map(|i| (tags!("test"), Payload::from_json_str(&i.to_string()).unwrap()))
            .collect::<Vec<_>>();
        store.append(app_id(), events).await?;

        Ok(store)
    }

    async fn test_retain_count(events_to_retain: u64) {
        let event_count = 1024;
        util::setup_logger();
        let test_stream = StreamNr::from(1);

        let store = publish_events(event_count).await.unwrap();
        let stream_id = store.node_id().stream(test_stream);

        let stream = store.get_or_create_own_stream(test_stream).unwrap();
        let guard = stream.lock().await;
        super::prune_stream(&store, guard, &RetainConfig::events(events_to_retain), Timestamp::now()).unwrap();

        let query = OffsetQuery::from(0..);
        let round_tripped = store
            .stream_filtered_chunked(stream_id, 0..=u64::MAX, query)
            .take_until_condition(|x| future::ready(x.as_ref().unwrap().range.end >= event_count))
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        for chunk in round_tripped {
            if chunk.range.end < event_count.saturating_sub(events_to_retain) {
                assert!(
                    chunk.data.is_empty(),
                    "Expected chunk data to be empty but it has data: {:?}",
                    chunk.data
                );
            } else {
                assert_eq!(
                    chunk.data.len(),
                    chunk.range.count(),
                    "Expected the same range, data: {:?}",
                    chunk.data
                );
            }
        }
    }

    #[tokio::test]
    async fn retain_count() {
        test_retain_count(u64::MAX).await;
        test_retain_count(1025).await;
        test_retain_count(1024).await;
        test_retain_count(1023).await;
        test_retain_count(512).await;
        test_retain_count(256).await;
        test_retain_count(1).await;
        test_retain_count(0).await;
    }

    async fn test_retain_size(max_size: u64) {
        let upper_bound = 1024;
        let test_stream = StreamNr::from(1);

        let store = publish_events(upper_bound).await.unwrap();

        let stream = store.get_or_create_own_stream(test_stream).unwrap();
        let guard = stream.lock().await;
        super::prune_stream(&store, guard, &RetainConfig::size(max_size), Timestamp::now()).unwrap();

        let query = OffsetQuery::from(0..);
        let round_tripped = store
            .stream_filtered_chunked(store.node_id().stream(test_stream), 0..=u64::MAX, query)
            .take_until_condition(|x| future::ready(x.as_ref().unwrap().range.end >= upper_bound))
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .rev()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut bytes = 0u64;
        for chunk in round_tripped {
            tracing::debug!(
                "-----\nmax_size {}\nbytes {}\nchunk.data.len() {}\n{:?}\n-----",
                max_size,
                bytes,
                chunk.data.len(),
                chunk
            );

            if bytes > max_size {
                assert!(chunk.data.is_empty());
            } else {
                bytes += chunk.data.len() as u64 * 4;
                assert_eq!(chunk.data.len(), chunk.range.count());
            }
        }
    }

    #[tokio::test]
    async fn retain_max_size() {
        test_retain_size(u64::MAX).await;
        test_retain_size(1025).await;
        test_retain_size(1024).await;
        test_retain_size(1023).await;
        test_retain_size(512).await;
        test_retain_size(256).await;
        test_retain_size(1).await;
        test_retain_size(0).await;
    }

    /// Publishes `event_count` events, and waits some time between each chunk.
    /// This introduces different time stamps into the persisted events.
    async fn publish_events_chunked(
        stream_nr: StreamNr,
        event_count: u64,
        base: Timestamp,
    ) -> anyhow::Result<BanyanStore> {
        let store = create_store().await?;
        let events = (0..event_count)
            .into_iter()
            .map(|i| (tags!("test"), Payload::from_json_str(&i.to_string()).unwrap()))
            .collect::<Vec<_>>();
        for (i, chunk) in events.chunks((event_count / 100) as usize).enumerate() {
            let timestamp = base + Duration::from_millis(i as u64);
            store.append0(stream_nr, app_id(), timestamp, chunk.to_vec()).await?;
        }

        Ok(store)
    }

    async fn test_retain_age(percentage_to_keep: usize) {
        util::setup_logger();
        let event_count = 1024;
        let max_leaf_count = SwarmConfig::test("..").banyan_config.tree.max_leaf_count as usize;
        let test_stream = StreamNr::from(1);

        let now = Timestamp::now();
        let store = publish_events_chunked(test_stream, event_count, now).await.unwrap();

        // Get actual timestamps from chunks
        let all_events = store
            .stream_filtered_chunked(
                store.node_id().stream(test_stream),
                0..=u64::MAX,
                OffsetQuery::from(0..),
            )
            .take_until_condition(|x| future::ready(x.as_ref().unwrap().range.end >= event_count))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        let first = all_events.first().map(|c| c.data.first().unwrap().1.time()).unwrap();
        let last = all_events.last().map(|c| c.data.first().unwrap().1.time()).unwrap();
        let dur = Duration::from_micros((percentage_to_keep * (last - first) as usize / 100) as u64);
        let cut_off = last - dur - Duration::from_micros(1);

        let events_to_keep = all_events.iter().fold(0, |acc, chunk| {
            let is_sealed = chunk.data.len() == max_leaf_count;
            if is_sealed && chunk.data.last().unwrap().1.time() <= cut_off {
                acc
            } else {
                acc + chunk.data.len()
            }
        });

        // Test this fn directly in order to avoid messing around with the `SystemTime`
        let stream = store.get_or_create_own_stream(test_stream).unwrap();
        let guard = stream.lock().await;
        super::prune_stream(
            &store,
            guard,
            &RetainConfig::age_from_millis(dur.as_millis() as u64),
            last,
        )
        .unwrap();

        let round_tripped = store
            .stream_filtered_chunked(
                store.node_id().stream(test_stream),
                0..=u64::MAX,
                OffsetQuery::from(0..),
            )
            .take_until_condition(|x| future::ready(x.as_ref().unwrap().range.end >= event_count))
            .flat_map(|chunk| {
                futures::stream::iter(match chunk {
                    Ok(chunk) => Either::Left(chunk.data.into_iter().map(Ok)),
                    Err(err) => Either::Right(once(Err(err))),
                })
            })
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        let mut expected = all_events
            .into_iter()
            .flat_map(|c| c.data)
            .rev()
            .take(events_to_keep)
            .collect::<Vec<_>>();
        expected.reverse();

        assert_eq!(expected.len(), round_tripped.len());
        assert_eq!(expected, round_tripped);
    }

    #[tokio::test]
    async fn retain_age() {
        test_retain_age(0).await;
        test_retain_age(25).await;
        test_retain_age(50).await;
        test_retain_age(75).await;
        test_retain_age(99).await;
        test_retain_age(100).await;
        test_retain_age(200).await;
    }
}
