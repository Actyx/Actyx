use crate::{streams::OwnStreamGuard, BanyanStore, EphemeralEventsConfig, Link};
use actyx_sdk::Timestamp;
use anyhow::Context;
use futures::future::{join_all, FutureExt};
use serde::{Deserialize, Serialize};
use std::{
    convert::TryInto,
    future,
    time::{Duration, SystemTime},
};
use trees::query::{OffsetQuery, TimeQuery};

/// Note: Events are kept on a best-effort basis, potentially violating the
/// constraints expressed by this config.
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RetainConfig {
    /// Retains the last `n` events.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_events: Option<u64>,
    /// Retain all events between `now - duration` and `now` (in seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<u64>,
    /// Retain the last events up to the provided size in bytes. Note that only
    /// the value bytes are taken into account, no overhead from keys, indexes,
    /// etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u64>,
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

    /// Limit the age of the events to keep (in seconds).
    pub fn age(age: u64) -> Self {
        Self {
            max_events: None,
            max_age: Some(age),
            max_size: None,
        }
    }

    /// Limit the total size of the events to keep (in bytes).
    pub fn size(size: u64) -> Self {
        Self {
            max_events: None,
            max_age: None,
            max_size: Some(size),
        }
    }
}

fn retain_last_events(store: &BanyanStore, stream: &mut OwnStreamGuard<'_>, keep: u64) -> anyhow::Result<Option<Link>> {
    let stream_nr = stream.stream_nr();
    store.transform_stream(stream, |txn, tree| {
        txn.pack(tree)?;
        let max = tree.count();
        let lower_bound = max.saturating_sub(keep);
        if lower_bound > 0 {
            let query = OffsetQuery::from(lower_bound..);
            tracing::debug!("Ephemeral events on {}; retain {:?}", stream_nr, query);
            txn.retain(tree, &query)?;
        }
        Ok(())
    })?;
    Ok(stream.snapshot().link())
}

fn retain_events_after(
    store: &BanyanStore,
    stream: &mut OwnStreamGuard<'_>,
    emit_after: Timestamp,
) -> anyhow::Result<Option<Link>> {
    let stream_nr = stream.stream_nr();
    store.transform_stream(stream, |txn, tree| {
        txn.pack(tree)?;
        let query = TimeQuery::from(emit_after..);
        tracing::debug!("Prune events on {}; retain {:?}", stream_nr, query);
        txn.retain(tree, &query)
    })?;
    Ok(stream.snapshot().link())
}

fn retain_events_up_to(
    store: &BanyanStore,
    stream: &mut OwnStreamGuard<'_>,
    target_bytes: u64,
) -> anyhow::Result<Option<Link>> {
    let stream_nr = stream.stream_nr();
    let emit_from = {
        let tree = stream.snapshot();
        let mut iter = store.data.forest.iter_index_reverse(&tree, banyan::query::AllQuery);
        let mut bytes = 0u64;
        let mut current_offset = tree.count();
        loop {
            if let Some(maybe_index) = iter.next() {
                let index = maybe_index?;
                // If we want to be a bit smarter here, we need to extend
                // `banyan` for a more elaborated traversal API. For now a plain
                // iterator is enough, and will be for a long time.
                if let banyan::index::Index::Leaf(l) = index {
                    // Only the value bytes are taken into account
                    bytes += l.value_bytes;
                    current_offset -= l.keys().count() as u64;
                    if bytes >= target_bytes {
                        tracing::debug!(
                            "Prune events on {}; hitting size target {} > {}. \
                            Results in min offset (non-inclusive) {}",
                            stream_nr,
                            bytes + l.value_bytes,
                            target_bytes,
                            current_offset
                        );
                        break current_offset;
                    }
                }
            } else {
                tracing::debug!(
                    "Prune events on {}; no change needed as tree size {} < {}",
                    stream_nr,
                    bytes,
                    target_bytes
                );
                break 0u64;
            }
        }
    };

    if emit_from > 0u64 {
        // lower bound is inclusive, so increment
        let query = OffsetQuery::from(emit_from..);
        store.transform_stream(stream, |txn, tree| {
            txn.pack(tree)?;
            tracing::debug!("Prune events on {}; retain {:?}", stream_nr, query);
            txn.retain(tree, &query)
        })?;
        Ok(stream.snapshot().link())
    } else {
        // No need to update the tree.
        // (Returned digest is not evaluated anyway)
        Ok(None)
    }
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
                let mut guard = stream.lock().await;
                let mut result = Ok(None);
                if let Some(keep) = cfg.max_events {
                    result = retain_last_events(&store, &mut guard, keep);
                }
                if let Some(duration) = cfg.max_age {
                    let emit_after: Timestamp = SystemTime::now()
                        .checked_sub(Duration::from_secs(duration))
                        .with_context(|| format!("Invalid duration configured for {}: {:?}", stream_nr, duration))?
                        .try_into()?;
                    result = retain_events_after(&store, &mut guard, emit_after);
                }
                if let Some(max_retain_size) = cfg.max_size {
                    result = retain_events_up_to(&store, &mut guard, max_retain_size);
                }
                result
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
    use futures::{future, StreamExt, TryStreamExt};

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
        let mut guard = stream.lock().await;
        super::retain_last_events(&store, &mut guard, events_to_retain).unwrap();

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
        let mut guard = stream.lock().await;
        super::retain_events_up_to(&store, &mut guard, max_size).unwrap();

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
        let event_count = 1024;
        let max_leaf_count = SwarmConfig::test("..").banyan_config.tree.max_leaf_count as usize;
        util::setup_logger();
        let test_stream = StreamNr::from(0);

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

        let cut_off = {
            let first = all_events.first().map(|c| c.data.first().unwrap().1.time()).unwrap();
            let last = all_events.last().map(|c| c.data.first().unwrap().1.time()).unwrap();
            let dur = Duration::from_micros((percentage_to_keep * (last - first) as usize / 100) as u64);
            now - dur - Duration::from_micros(1)
        };
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
        let mut guard = stream.lock().await;
        super::retain_events_after(&store, &mut guard, cut_off).unwrap();

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
