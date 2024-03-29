use crate::{
    ax_futures_util::stream::{interval, AxStreamExt, Drainer},
    crypto::{KeyPair, KeyStore, PublicKey},
    swarm::{
        AxTreeExt, BanyanStore, EphemeralEventsConfig, EventRoute, EventRouteMappingEvent, SwarmConfig,
        DEFAULT_STREAM_NAME, DISCOVERY_STREAM_NAME, FILES_STREAM_NAME, MAX_TREE_LEVEL, METRICS_STREAM_NAME,
    },
    trees::query::TagExprQuery,
};
use acto::ActoRef;
use anyhow::Result;
use ax_aql::TagExpr;
use ax_types::{app_id, tags, AppId, Offset, OffsetMap, Payload, StreamNr, Tag, TagSet};
use banyan::query::AllQuery;
use futures::{pin_mut, prelude::*, StreamExt};
use libipld::Cid;
use maplit::btreemap;
use std::{
    collections::BTreeMap,
    convert::TryFrom,
    fs, io,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};
use tokio::runtime::Runtime;
struct Tagger(BTreeMap<&'static str, Tag>);

impl Tagger {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn tag(&mut self, name: &'static str) -> Tag {
        self.0
            .entry(name)
            .or_insert_with(|| Tag::from_str(name).unwrap())
            .clone()
    }

    pub fn tags(&mut self, names: &[&'static str]) -> TagSet {
        names.iter().map(|name| self.tag(name)).collect::<TagSet>()
    }
}

fn app_id() -> AppId {
    app_id!("test")
}

#[allow(dead_code)]
fn cids_to_string(cids: Vec<Cid>) -> String {
    cids.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

#[tokio::test]
#[ignore]
async fn smoke() -> Result<()> {
    crate::util::setup_logger();
    let mut tagger = Tagger::new();
    let mut ev = move |tag| (tagger.tags(&[tag]), Payload::null());
    let store = BanyanStore::test("smoke").await?;
    let mut ipfs = store.ipfs().clone();
    tokio::task::spawn(store.stream_filtered_stream_ordered(AllQuery).for_each(|x| {
        tracing::info!("got event {:?}", x);
        future::ready(())
    }));
    tracing::info!("append first event!");
    let _ = store.append(app_id(), vec![ev("a")]).await?;
    tracing::info!("append second event!");
    tokio::task::spawn(interval(Duration::from_secs(1)).for_each(move |_| {
        let store = store.clone();
        let mut tagger = Tagger::new();
        let mut ev = move |tag| (tagger.tags(&[tag]), Payload::null());
        async move {
            let _ = store.append(app_id(), vec![ev("a")]).await.unwrap();
        }
    }));
    tokio::task::spawn(async move {
        ipfs.subscribe("test".to_owned())
            .await
            .unwrap()
            .for_each(|msg| {
                tracing::error!("event {:?}", msg);
                future::ready(())
            })
            .await
    });
    tokio::time::sleep(Duration::from_secs(1000)).await;
    Ok(())
}

fn last_item<T: Clone>(drainer: &mut Drainer<T>) -> anyhow::Result<T> {
    let mut vec = drainer.next().ok_or_else(|| anyhow::anyhow!("Stream ended"))?;
    vec.pop().ok_or_else(|| anyhow::anyhow!("Stream returned pending"))
}

#[tokio::test]
async fn should_compact() {
    // this will take 1010 chunks, so it will hit the MAX_TREE_LEVEL limit once
    const EVENTS: usize = 10100;
    let stream_nr = StreamNr::from(1);
    let mut config = SwarmConfig::test_with_routing(
        "compaction_interval",
        vec![EventRoute::new(
            TagExpr::from_str("'abc'").unwrap(),
            "test_stream".to_string(),
        )],
    );
    config.cadence_compact = Duration::from_secs(100000);
    let store = BanyanStore::new(config, ActoRef::blackhole()).await.unwrap();

    // Wait for the first compaction loop to pass.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let stream = store.get_or_create_own_stream(stream_nr).unwrap();
    let tree_stream = stream.tree_stream();
    let mut tree_stream = Drainer::new(tree_stream);
    assert_eq!(last_item(&mut tree_stream).unwrap().count(), 0);

    // Chunk to force creation of new branches
    for chunk in (0..EVENTS)
        .map(|_| (tags!("abc"), Payload::null()))
        .collect::<Vec<_>>()
        .chunks(10)
    {
        store.append(app_id(), chunk.to_vec()).await.unwrap();
    }
    let tree_after_append = last_item(&mut tree_stream).unwrap();
    assert!(!store.data.forest.is_packed(&tree_after_append).unwrap());

    // get the events back
    let evs = store
        .stream_filtered_chunked(store.node_id().stream(stream_nr), 0..=u64::MAX, AllQuery)
        .take_until_signaled(tokio::time::sleep(Duration::from_secs(2)))
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()
        .unwrap()
        .into_iter()
        .flat_map(|c| c.data);
    assert_eq!(evs.count(), EVENTS);
    // Make sure the root didn't change
    let empty = tree_stream.next().unwrap();
    assert!(empty.is_empty(), "{:?}", empty);

    // running compaction manually here to make the test deterministic
    let mut guard = stream.lock().await;
    store.transform_stream(&mut guard, |txn, tree| txn.pack(tree)).unwrap();
    drop(guard);

    let tree_after_compaction = last_item(&mut tree_stream).unwrap();
    assert_ne!(tree_after_append.root(), tree_after_compaction.root());
    assert!(store.data.forest.is_packed(&tree_after_compaction).unwrap());
}

#[tokio::test]
async fn should_extend_packed_when_hitting_max_tree_depth() {
    let store = BanyanStore::test_with_routing(
        "compaction_max_tree",
        vec![EventRoute::new(
            TagExpr::from_str("'abc'").unwrap(),
            "test_stream".to_string(),
        )],
    )
    .await
    .unwrap();

    // Wait for the first compaction loop to pass.
    tokio::time::sleep(Duration::from_millis(500)).await;

    let tree_stream = store.get_or_create_own_stream(1.into()).unwrap().tree_stream();
    let mut tree_stream = Drainer::new(tree_stream);
    assert_eq!(last_item(&mut tree_stream).unwrap().count(), 0);

    // Append individually to force creation of new branches
    // -1 because of the `default` mapping event
    for ev in (0..MAX_TREE_LEVEL).map(|_| (tags!("abc"), Payload::null())) {
        store.append(app_id(), vec![ev]).await.unwrap();
    }
    let tree_after_append = last_item(&mut tree_stream).unwrap();
    assert!(!store.data.forest.is_packed(&tree_after_append).unwrap());
    assert_eq!(tree_after_append.level(), MAX_TREE_LEVEL);
    assert_eq!(
        tree_after_append.offset(),
        Some(Offset::try_from((MAX_TREE_LEVEL - 1) as i64).unwrap())
    );

    // packing will be triggered when the existing tree's level is MAX_TREE_LEVEL + 1
    store
        .append(app_id(), vec![(tags!("abc"), Payload::null())])
        .await
        .unwrap();
    let tree_after_pack = last_item(&mut tree_stream).unwrap();
    // the tree is not packed
    assert!(store.data.forest.is_packed(&tree_after_pack).unwrap());
    // but the max level remains constant now
    assert_eq!(tree_after_pack.level(), 3);
    assert_eq!(
        tree_after_pack.offset(),
        Some(Offset::try_from(MAX_TREE_LEVEL as i64).unwrap())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn must_not_lose_events_through_compaction() -> Result<()> {
    const EVENTS: usize = 1000;
    let store = BanyanStore::test("compaction_max_tree").await?;
    // compact continuously
    store.spawn_task(
        "compaction".to_owned(),
        store.clone().compaction_loop(Duration::from_micros(0)).boxed(),
    );

    let tags_query =
        TagExprQuery::from_expr(&"'abc'".parse().unwrap()).unwrap()(true, store.node_id().stream(0.into()));

    for ev in (0..EVENTS).map(|_| (tags!("abc"), Payload::null())) {
        store.append(app_id(), vec![ev]).await?;
    }

    let evs = store
        .stream_filtered_stream_ordered(tags_query)
        .take(EVENTS)
        .take_until_signaled(tokio::time::sleep(Duration::from_secs(2)))
        .map_ok(|x| x.0)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<anyhow::Result<Vec<_>>>()?;
    anyhow::ensure!(
        evs.len() == EVENTS,
        "Expected {} events, but only got back {}. Received: {:?}",
        EVENTS,
        evs.len(),
        evs
    );

    Ok(())
}

fn config_in_temp_folder() -> anyhow::Result<(SwarmConfig, tempfile::TempDir)> {
    let dir = tempfile::tempdir()?;
    let db = PathBuf::from(dir.path().join("db").to_str().expect("illegal filename"));
    let index = PathBuf::from(dir.path().join("index").to_str().expect("illegal filename"));
    let config = SwarmConfig {
        index_store: Some(index),
        node_name: Some("must_report_proper_initial_offsets".to_owned()),
        db_path: Some(db),
        enable_mdns: false,
        keypair: Some(KeyPair::generate()),
        ..SwarmConfig::basic()
    };
    Ok((config, dir))
}

#[tokio::test]
async fn must_report_proper_initial_offsets() {
    const EVENTS: usize = 10;
    let (mut config, _dir) = config_in_temp_folder().unwrap();
    config.event_routes = vec![EventRoute::new(
        TagExpr::from_str("'abc'").unwrap(),
        "extra".to_string(),
    )];
    let store = BanyanStore::new(config.clone(), ActoRef::blackhole()).await.unwrap();
    let expected_present = OffsetMap::from(btreemap! {
        store.node_id().stream(0.into()) => Offset::from(1),
        store.node_id().stream(1.into()) => Offset::from(9)
    });

    for ev in (0..EVENTS).map(|_| (tags!("abc"), Payload::null())) {
        store.append(app_id(), vec![ev]).await.unwrap();
    }

    let present = store.data.offsets.project(|x| x.present.clone());
    assert_eq!(present, expected_present);
    drop(store);

    // load non-empty store from disk and check that the offsets are correctly computed
    let store = BanyanStore::new(config, ActoRef::blackhole()).await.unwrap();
    let swarm_offsets = store.data.offsets.project(Clone::clone);
    assert_eq!(swarm_offsets.present, expected_present);
    // replication_target should be equal to the present. is nulled in the event service API
    assert_eq!(swarm_offsets.replication_target, expected_present);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_add_cat() -> Result<()> {
    use rand::RngCore;
    crate::util::setup_logger();
    let store = BanyanStore::test("local").await?;
    let mut data = vec![0; 16_000_000];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut data);
    let mut tmp = store.ipfs().create_temp_pin()?;
    let (root, _) = store.add(&mut tmp, &data[..])?;
    let mut buf = Vec::with_capacity(16_000_000);
    let stream = store.cat(root, true);
    pin_mut!(stream);
    while let Some(res) = stream.next().await {
        let mut bytes = res?;
        buf.append(&mut bytes);
    }
    assert_eq!(buf, data);
    Ok(())
}

#[test]
fn test_add_zero_bytes() -> Result<()> {
    let rt = Runtime::new()?;
    rt.block_on(async {
        crate::util::setup_logger();
        let store = BanyanStore::test("local").await?;
        tracing::info!("store created");
        let mut tmp = store.ipfs().create_temp_pin()?;
        tracing::info!("temp pin created");
        let data: &[u8] = &[];
        store.add(&mut tmp, data)?;
        tracing::info!("data added");
        drop(tmp);
        tracing::info!("temp pin dropped");
        drop(store); // without this the test sometimes doesn’t complete
        tracing::info!("store dropped");
        Ok(())
    })
}

/// Emulates a fresh swarm launch from an empty config (i.e. nodes after 2.15).
/// Expected streams should be "default", "metrics", "discovery", "files".
#[tokio::test]
async fn non_existing_swarm_config() {
    crate::util::setup_logger();

    let dir = tempfile::tempdir().unwrap();
    let db = PathBuf::from(dir.path().join("db").to_str().expect("illegal filename"));
    let index = PathBuf::from(dir.path().join("index").to_str().expect("illegal filename"));

    let config = SwarmConfig {
        index_store: Some(index),
        db_path: Some(db),
        ..SwarmConfig::basic()
    };

    let store = BanyanStore::new(config, ActoRef::blackhole()).await.unwrap();

    let expected_mappings = vec![
        EventRouteMappingEvent {
            stream_name: DEFAULT_STREAM_NAME.to_string(),
            stream_nr: 0.into(),
        },
        EventRouteMappingEvent {
            stream_name: DISCOVERY_STREAM_NAME.to_string(),
            stream_nr: 1.into(),
        },
        EventRouteMappingEvent {
            stream_name: METRICS_STREAM_NAME.to_string(),
            stream_nr: 2.into(),
        },
        EventRouteMappingEvent {
            stream_name: FILES_STREAM_NAME.to_string(),
            stream_nr: 3.into(),
        },
    ];

    let tree_level = store
        .get_or_create_own_stream(0.into())
        .unwrap()
        .tree_stream()
        .next()
        .await
        .unwrap()
        .level();

    let mut round_tripped = store
        .stream_filtered_chunked(store.node_id().stream(0.into()), 0..=u64::MAX, AllQuery)
        .take_until_condition(|x| future::ready(x.as_ref().unwrap().range.end >= tree_level as u64))
        .map(|chunk| chunk.unwrap().data)
        .flat_map(|a| {
            stream::iter(
                a.into_iter()
                    .map(|(_, _, event)| event.extract::<EventRouteMappingEvent>().map_err(anyhow::Error::from)),
            )
        })
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
    round_tripped.sort_by_key(|event| event.stream_nr);

    assert_eq!(round_tripped.len(), expected_mappings.len());
    for i in 0..expected_mappings.len() {
        assert_eq!(expected_mappings[i], round_tripped[i]);
    }
}

/// Emulates changing to a new topic, using an existing configuration.
/// Expected streams should be the default one and any additional streams.
#[tokio::test]
async fn existing_swarm_config() {
    let dir = tempfile::tempdir().unwrap();
    let db = PathBuf::from(dir.path().join("db").to_str().expect("illegal filename"));
    let index = PathBuf::from(dir.path().join("index").to_str().expect("illegal filename"));

    let config = SwarmConfig {
        index_store: Some(index),
        db_path: Some(db),
        topic: "test-topic".to_string(),
        ephemeral_event_config: EphemeralEventsConfig {
            streams: btreemap! {
                "stream_1".to_string() => Default::default(),
                "stream_2".to_string() => Default::default(),
                // Stream 3 should not be allocated and generate a warning instead
                "stream_3".to_string() => Default::default(),
            },
            ..Default::default()
        },
        event_routes: vec![
            EventRoute::new(TagExpr::from_str("allEvents").unwrap(), "default".to_string()),
            EventRoute::new(TagExpr::from_str("'stream_1'").unwrap(), "stream_1".to_string()),
            EventRoute::new(TagExpr::from_str("'stream_2'").unwrap(), "stream_2".to_string()),
        ],
        ..SwarmConfig::basic()
    };
    let store = BanyanStore::new(config, ActoRef::blackhole()).await.unwrap();

    let expected_mappings = vec![
        EventRouteMappingEvent {
            stream_name: "default".to_string(),
            stream_nr: 0.into(),
        },
        EventRouteMappingEvent {
            stream_name: "stream_1".to_string(),
            stream_nr: 1.into(),
        },
        EventRouteMappingEvent {
            stream_name: "stream_2".to_string(),
            stream_nr: 2.into(),
        },
    ];

    let tree_level = store
        .get_or_create_own_stream(0.into())
        .unwrap()
        .tree_stream()
        .next()
        .await
        .unwrap()
        .level();

    let mut round_tripped = store
        .stream_filtered_chunked(store.node_id().stream(0.into()), 0..=u64::MAX, AllQuery)
        .take_until_condition(|x| future::ready(x.as_ref().unwrap().range.end >= tree_level as u64))
        .map(|chunk| chunk.unwrap().data)
        .flat_map(|a| {
            stream::iter(
                a.into_iter()
                    .map(|(_, _, event)| event.extract::<EventRouteMappingEvent>().map_err(anyhow::Error::from)),
            )
        })
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
    round_tripped.sort_by_key(|event| event.stream_nr);

    assert_eq!(round_tripped.len(), expected_mappings.len());
    for i in 0..expected_mappings.len() {
        assert_eq!(expected_mappings[i], round_tripped[i]);
    }
}

fn copy_dir_recursive(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_recursive(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn get_keypair() -> KeyPair {
    let keystore_str = "AWcltN48gptE+LELhtwVNV3+yDfu5BkvA+u1cQO6VrHwFDQUBwhVOuD7XZRe5MOlHfCFRh2Ye7TJUAwl8YJSRZLZH/NZxE9ErvHTcDIWdyIvFnenIjJfSukahnBPVdwwOgqBo8QhxneO0z99ug9p54XXtlL87V0leWto2QjIup29KWXne9QanHk4oA==";
    let keystore = KeyStore::restore(io::Cursor::new(base64::decode(keystore_str).unwrap())).unwrap();
    let _p = keystore.get_pairs();
    keystore
        .get_pair(PublicKey::from_str("01CANwnBSbmTlT3HWrTVZvfTPSeRZuGfcOJnF7Z/X6Nc=").unwrap())
        .unwrap()
}

/// Emulates a swarm launch from a node previous to 2.15.
/// Expected streams are the "default", "discovery", "metrics" and "files".
#[tokio::test]
async fn non_existing_swarm_config_existing_streams() {
    use tempfile::TempDir;

    crate::util::setup_logger();

    let expected_mappings = vec![
        EventRouteMappingEvent {
            stream_name: "default".to_string(),
            stream_nr: 0.into(),
        },
        EventRouteMappingEvent {
            stream_name: "discovery".to_string(),
            stream_nr: 1.into(),
        },
        EventRouteMappingEvent {
            stream_name: "metrics".to_string(),
            stream_nr: 2.into(),
        },
        EventRouteMappingEvent {
            stream_name: "files".to_string(),
            stream_nr: 3.into(),
        },
    ];

    let dir = PathBuf::from_str("test-data/v2.15").unwrap();
    let temp_dir = TempDir::new().unwrap();

    copy_dir_recursive(dir, temp_dir.path()).unwrap();

    let blobs = PathBuf::from(
        temp_dir
            .path()
            .join("store/test-blobs.sqlite")
            .to_str()
            .expect("illegal filename"),
    );
    let index = PathBuf::from(
        temp_dir
            .path()
            .join("store/test-index.sqlite")
            .to_str()
            .expect("illegal filename"),
    );
    let db = PathBuf::from(temp_dir.path().join("node.sqlite").to_str().expect("illegal filename"));

    let config = SwarmConfig {
        keypair: Some(get_keypair()),
        index_store: Some(index.clone()),
        blob_store: Some(blobs.clone()),
        db_path: Some(db.clone()),
        ..SwarmConfig::basic()
    };

    let store = BanyanStore::new(config, ActoRef::blackhole()).await.unwrap();

    let tree_level = store
        .get_or_create_own_stream(0.into())
        .unwrap()
        .tree_stream()
        .next()
        .await
        .unwrap()
        .level();

    let mut round_tripped = store
        .stream_filtered_chunked(store.node_id().stream(0.into()), 0..=u64::MAX, AllQuery)
        .take_until_condition(|x| future::ready(x.as_ref().unwrap().range.end >= tree_level as u64))
        .map(|chunk| chunk.unwrap().data)
        .flat_map(|a| {
            stream::iter(
                a.into_iter()
                    .map(|(_, _, event)| event.extract::<EventRouteMappingEvent>().map_err(anyhow::Error::from)),
            )
        })
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
    round_tripped.sort_by_key(|event| event.stream_nr);
    tracing::error!("{:?}", round_tripped);
    assert_eq!(round_tripped.len(), expected_mappings.len());
    for i in 0..expected_mappings.len() {
        assert_eq!(expected_mappings[i], round_tripped[i]);
    }

    drop(store);

    let config = SwarmConfig {
        keypair: Some(get_keypair()),
        topic: "other-topic".to_string(),
        index_store: Some(index),
        blob_store: Some(blobs),
        db_path: Some(db),
        ..SwarmConfig::basic()
    };

    let store = BanyanStore::new(config, ActoRef::blackhole()).await.unwrap();

    let tree_level = store
        .get_or_create_own_stream(0.into())
        .unwrap()
        .tree_stream()
        .next()
        .await
        .unwrap()
        .level();

    let mut round_tripped = store
        .stream_filtered_chunked(store.node_id().stream(0.into()), 0..=u64::MAX, AllQuery)
        .take_until_condition(|x| future::ready(x.as_ref().unwrap().range.end >= tree_level as u64))
        .map(|chunk| chunk.unwrap().data)
        .flat_map(|a| {
            stream::iter(
                a.into_iter()
                    .map(|(_, _, event)| event.extract::<EventRouteMappingEvent>().map_err(anyhow::Error::from)),
            )
        })
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
    round_tripped.sort_by_key(|event| event.stream_nr);
    tracing::error!("{:?}", round_tripped);
    assert_eq!(round_tripped.len(), expected_mappings.len());
    for i in 0..expected_mappings.len() {
        assert_eq!(expected_mappings[i], round_tripped[i]);
    }
}

/// Emulates a swarm launch from a node previous to 2.15 using a configuration for the second topic.
/// Expected streams are the "default", "discovery", "metrics" and "files".
#[tokio::test]
async fn existing_swarm_config_existing_streams() {
    use tempfile::TempDir;

    crate::util::setup_logger();
    println!("{:?}", std::env::current_dir().unwrap());
    // Copy the test data to a temporary directory because the tests modify the stores
    let dir = PathBuf::from_str("test-data/v2.15").unwrap();
    let temp_dir = TempDir::new().unwrap();
    copy_dir_recursive(dir, temp_dir.path()).unwrap();

    let expected_default_mappings = vec![
        EventRouteMappingEvent {
            stream_name: "default".to_string(),
            stream_nr: 0.into(),
        },
        EventRouteMappingEvent {
            stream_name: "discovery".to_string(),
            stream_nr: 1.into(),
        },
        EventRouteMappingEvent {
            stream_name: "metrics".to_string(),
            stream_nr: 2.into(),
        },
        EventRouteMappingEvent {
            stream_name: "files".to_string(),
            stream_nr: 3.into(),
        },
    ];

    let blobs = PathBuf::from(
        temp_dir
            .path()
            .join("store/test-blobs.sqlite")
            .to_str()
            .expect("illegal filename"),
    );
    let index = PathBuf::from(
        temp_dir
            .path()
            .join("store/test-index.sqlite")
            .to_str()
            .expect("illegal filename"),
    );
    let db = PathBuf::from(temp_dir.path().join("node.sqlite").to_str().expect("illegal filename"));

    let default_topic_config = SwarmConfig {
        keypair: Some(get_keypair()),
        index_store: Some(index.clone()),
        blob_store: Some(blobs.clone()),
        db_path: Some(db.clone()),
        ..SwarmConfig::basic()
    };

    let store = BanyanStore::new(default_topic_config, ActoRef::blackhole())
        .await
        .unwrap();

    let offset = store
        .get_or_create_own_stream(0.into())
        .unwrap()
        .published_tree()
        .unwrap()
        .offset();

    let mut round_tripped = store
        .stream_filtered_chunked(store.node_id().stream(0.into()), 0..=offset.into(), AllQuery)
        .map(|chunk| chunk.unwrap().data)
        .flat_map(|a| {
            stream::iter(
                a.into_iter()
                    .map(|(_, _, event)| event.extract::<EventRouteMappingEvent>().map_err(anyhow::Error::from)),
            )
        })
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
    round_tripped.sort_by_key(|event| event.stream_nr);
    tracing::error!("{:?}", round_tripped);
    assert_eq!(round_tripped.len(), expected_default_mappings.len());
    for i in 0..expected_default_mappings.len() {
        assert_eq!(expected_default_mappings[i], round_tripped[i]);
    }

    drop(store);

    let other_topic_config = SwarmConfig {
        keypair: Some(get_keypair()),
        topic: "other-topic".to_string(),
        index_store: Some(index),
        blob_store: Some(blobs),
        db_path: Some(db),
        event_routes: vec![EventRoute::new(
            TagExpr::from_str("'a'").unwrap(),
            "other-stream".to_string(),
        )],
        ..SwarmConfig::basic()
    };

    let expected_other_mappings = vec![
        EventRouteMappingEvent {
            stream_name: "default".to_string(),
            stream_nr: 0.into(),
        },
        EventRouteMappingEvent {
            stream_name: "discovery".to_string(),
            stream_nr: 1.into(),
        },
        EventRouteMappingEvent {
            stream_name: "metrics".to_string(),
            stream_nr: 2.into(),
        },
        EventRouteMappingEvent {
            stream_name: "files".to_string(),
            stream_nr: 3.into(),
        },
        EventRouteMappingEvent {
            stream_name: "other-stream".to_string(),
            stream_nr: 4.into(),
        },
    ];

    let store = BanyanStore::new(other_topic_config, ActoRef::blackhole())
        .await
        .unwrap();

    let offset = store
        .get_or_create_own_stream(0.into())
        .unwrap()
        .published_tree()
        .unwrap()
        .offset();

    let mut round_tripped = store
        .stream_filtered_chunked(store.node_id().stream(0.into()), 0..=offset.into(), AllQuery)
        .map(|chunk| chunk.unwrap().data)
        .flat_map(|a| {
            stream::iter(
                a.into_iter()
                    .map(|(_, _, event)| event.extract::<EventRouteMappingEvent>().map_err(anyhow::Error::from)),
            )
        })
        .try_collect::<Vec<_>>()
        .await
        .unwrap();
    round_tripped.sort_by_key(|event| event.stream_nr);
    tracing::error!("{:?}", round_tripped);
    assert_eq!(round_tripped.len(), expected_other_mappings.len());
    for i in 0..expected_other_mappings.len() {
        assert_eq!(expected_other_mappings[i], round_tripped[i]);
    }
}
