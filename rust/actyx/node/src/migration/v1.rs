use actyx_sdk::legacy::SourceId;
use anyhow::{bail, Context};
use flate2::{write::GzEncoder, Compression};
use ipfs_sqlite_block_store::BlockStore;
use parking_lot::Mutex;
use rusqlite::Connection;
use std::{
    collections::BTreeSet,
    fs, io,
    net::{Ipv4Addr, TcpListener},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use swarm::convert::ConversionOptions;
use tempfile::{tempdir_in, NamedTempFile};

use crate::{
    host::apply_system_schema,
    migration::{open_readonly, NODE_DB_FILENAME},
    node_storage::NodeStorage,
};

#[allow(dead_code)]
pub struct V1Directory {
    // base path
    path: PathBuf,
    // joined path to configured topic's index store
    pub index_db: PathBuf,
    // joined path to configured topic's blocks db
    blocks_db: PathBuf,
    // joined path to settings db
    settings_db: PathBuf,
    // Opened read-only
    pub settings_repo: settings::Repository,
    // joined path to node db
    node_db: PathBuf,
    // Opened read-only
    node_storage: NodeStorage,
    // configured topic
    pub topic: String,
}

/// Asserts the expected v1 directory layout, and returns a convenience struct
/// to access it.
pub fn assert_v1(v1_working_dir: impl AsRef<Path>) -> anyhow::Result<V1Directory> {
    // .
    // ├── 1.1.5
    // │   ├── logsvcd.sqlite
    // │   ├── node.sqlite
    // │   ├── .settings.db
    // │   └── store
    // │       ├── default-topic
    // │       └── default-topic-blocks.sqlite
    // └── 2.0
    //     ├── logsvcd.sqlite
    //     ├── node.sqlite
    //     ├── settings.db
    //     └── store
    //         ├── default-topic.sqlite
    //
    // On v1 docker all dbs (except logsvcd.sqlite) are created under
    // `<working_dir>/apps`:
    // .
    // ├── apps
    // │   ├── node.sqlite
    // │   ├── .settings.db
    // ├── logsvcd.sqlite
    // └── store
    //     ├── default-topic
    //     ├── default-topic-blocks.sqlite
    let db_dir: PathBuf = {
        let apps_dir = "apps";
        let with_apps = v1_working_dir.as_ref().join(apps_dir);
        if with_apps.is_dir() {
            if with_apps.join(NODE_DB_FILENAME).exists() {
                with_apps
            } else {
                v1_working_dir.as_ref().into()
            }
        } else {
            v1_working_dir.as_ref().into()
        }
    };
    let node_db = db_dir.join(NODE_DB_FILENAME);
    let connection = open_readonly(&node_db).context("Opening node.sqlite")?;
    anyhow::ensure!(
        NodeStorage::version(&connection)? <= 1,
        "Version of node.sqlite is not <= 1"
    );
    let node_storage = NodeStorage {
        connection: Arc::new(Mutex::new(connection)),
    };
    let settings_db = db_dir.join(".settings.db");
    let settings_repo = settings::Repository::new(settings::Database::from_connection_without_init(
        open_readonly(&settings_db).context("Opening .settings.db")?,
    ));
    let topic = settings_repo
        .get_settings(&"com.actyx.os/services/eventService/topic".parse().unwrap(), false)
        .context("Getting topic from old settings db")?
        .as_str()
        .unwrap()
        .to_string()
        .replace('/', "_");

    let store_dir = v1_working_dir.as_ref().join("store");
    let index_db = store_dir.join(&topic);
    let blocks_db = store_dir.join(format!("{}-blocks.sqlite", topic));

    anyhow::ensure!(
        std::fs::metadata(&index_db).is_ok(),
        "index DB {} does not exist",
        index_db.display()
    );
    anyhow::ensure!(
        std::fs::metadata(&blocks_db).is_ok(),
        "blocks DB {} does not exist",
        blocks_db.display()
    );
    Ok(V1Directory {
        path: v1_working_dir.as_ref().into(),
        blocks_db,
        index_db,
        node_storage,
        node_db,
        topic,
        settings_db,
        settings_repo,
    })
}

/// Creates a new DB from a given [`rusqlite::Connection`] at `to`.
fn copy_db(from: &Connection, to: impl AsRef<Path>) -> anyhow::Result<Connection> {
    let mut to = Connection::open(to)?;
    let backup = rusqlite::backup::Backup::new(from, &mut to)?;
    backup.run_to_completion(1000, Duration::from_millis(1), None)?;
    std::mem::drop(backup);
    Ok(to)
}

/// Migrates settings from a given `old` repository to a new one, which is
/// created within `new_in_dir`.
fn migrate_settings(old: &settings::Repository, new_in_dir: impl AsRef<Path>) -> anyhow::Result<()> {
    let mut new = settings::Repository::new(settings::Database::new(new_in_dir.as_ref().to_path_buf())?);
    apply_system_schema(&mut new)?;

    for (source, target) in [
        ("com.actyx.os/services/eventService/topic", "com.actyx/swarm/topic"),
        (
            "com.actyx.os/services/eventService/readOnly",
            "com.actyx/api/events/readOnly",
        ),
        ("com.actyx.os/general/displayName", "com.actyx/admin/displayName"),
    ]
    .iter()
    {
        let val = old.get_settings(&source.parse().unwrap(), false)?;
        new.update_settings(&target.parse().unwrap(), val, false)?;
    }

    // convert potentially legacy multiaddrs
    for (source, target) in [
        ("com.actyx.os/general/bootstrapNodes", "com.actyx/swarm/initialPeers"),
        (
            "com.actyx.os/general/announceAddresses",
            "com.actyx/swarm/announceAddresses",
        ),
    ]
    .iter()
    {
        let mut val = old.get_settings(&source.parse().unwrap(), false)?;
        for v in val.as_array_mut().unwrap() {
            *v = serde_json::Value::String(v.as_str().unwrap().replace("/ipfs/", "/p2p/"));
        }
        new.update_settings(&target.parse().unwrap(), val, false)?;
    }
    let v1_swarm_key = old.get_settings(&"com.actyx.os/general/swarmKey".parse().unwrap(), false)?;
    let v2_swarm_key = convert_swarm_key_v1_v2(v1_swarm_key.as_str().unwrap())?;
    new.update_settings(
        &"com.actyx/swarm/swarmKey".parse().unwrap(),
        serde_json::Value::String(v2_swarm_key),
        false,
    )?;

    Ok(())
}
pub fn convert_swarm_key_v1_v2(v1_swarm_key: &str) -> anyhow::Result<String> {
    let key: String = String::from_utf8(base64::decode(v1_swarm_key).unwrap()).unwrap();
    let key: Vec<&str> = key.lines().collect();
    anyhow::ensure!(key.len() == 3, "Invalid swarm key format");
    let x = hex::decode(key[2]).unwrap();
    let v2_swarm_key = base64::encode(&x);
    anyhow::ensure!(v2_swarm_key.len() == 44, "Invalid swarm key format");
    Ok(v2_swarm_key)
}

/// Check whether v1 is currently running or not. v1 didn't allow providing
/// custom ports, so we can leverage that fact.
fn is_v1_running() -> bool {
    [4001, 4243, 4454, 4457, 5001]
        .iter()
        .copied()
        .any(is_port_bound_on_localhost)
}

fn is_port_bound_on_localhost(port: u16) -> bool {
    // only check IpV4, as v1 only never worked with IpV6 anyway.
    // Furthermore, IpV6 is not supported by default in Docker.
    TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_err()
}

/// Creates a gzipped tar archive with the following files form a given `v1`
/// dir:
///   * node.sqlite
///   * .settings.db
///   * index store and blocks db for the configured(!) topic
/// The resulting file will be placed inside `backup_in`.
fn backup_v1(v1: &V1Directory, backup_in: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
    let unix_ts = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let f = {
        let tmp_file = NamedTempFile::new_in(backup_in.as_ref())?;
        let mut archive = tar::Builder::new(GzEncoder::new(tmp_file, Compression::default()));
        for p in [&v1.node_db, &v1.settings_db].iter() {
            archive
                .append_path_with_name(&p, p.file_name().unwrap().to_string_lossy().to_string())
                .with_context(|| format!("Appending {} to archive", p.display()))?;
        }
        let store_dir = v1.index_db.parent().unwrap();
        archive
            .append_dir_all("store", &store_dir)
            .with_context(|| format!("Appending {} to archive", store_dir.display()))?;
        archive
            .into_inner()
            .and_then(|x| x.finish())
            .context("Finishing archive")?
    };
    let p = backup_in.as_ref().join(format!("{}_v1_data_files.tar.gz", unix_ts));
    if cfg!(target_os = "android") {
        // Android doesn't support creating hard links. At all.
        let (_, path) = f.keep()?;
        fs::copy(path, &p)?;
    } else {
        let _ = f.persist_noclobber(&p)?;
    }
    Ok(p)
}

/// Removes `node.sqlite`, `.settings.db`, and the `store` dir.
fn remove_v1(v1: V1Directory) -> anyhow::Result<()> {
    let V1Directory {
        node_db,
        settings_db,
        index_db,
        node_storage,
        settings_repo,
        ..
    } = v1;
    // it seems the `..` syntax doesn't lead to Drop being immediately
    // executed?!
    drop(node_storage);
    drop(settings_repo);
    // Also delete -wal and -shm files, if any
    for p in [settings_db, node_db].iter().flat_map(|db| {
        if let Some(ext) = db.extension() {
            let mut shm = db.clone();
            let mut shm_ext = ext.to_owned();
            shm_ext.push("-shm");
            shm.set_extension(shm_ext);

            let mut wal = db.clone();
            let mut wal_ext = ext.to_owned();
            wal_ext.push("-wal");
            wal.set_extension(wal_ext);

            vec![db.clone(), shm, wal]
        } else if let Some(file_name) = db.file_name() {
            let mut shm = db.clone();
            let mut shm_file_name = file_name.to_owned();
            shm_file_name.push("-shm");
            shm.set_file_name(shm_file_name);

            let mut wal = db.clone();
            let mut wal_file_name = file_name.to_owned();
            wal_file_name.push("-wal");
            wal.set_file_name(wal_file_name);

            vec![db.clone(), shm, wal]
        } else {
            unreachable!()
        }
    }) {
        match fs::remove_file(&p) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            e => e.with_context(|| format!("Removing {}", p.display()))?,
        }
    }
    let store_dir = index_db.parent().unwrap();
    fs::remove_dir_all(&store_dir).with_context(|| format!("Removing {}", store_dir.display()))?;
    Ok(())
}

fn rename_recursive(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> anyhow::Result<()> {
    fs::create_dir_all(&dst).with_context(|| format!("Creating {}", dst.as_ref().display()))?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let to = dst.as_ref().join(entry.file_name());
        fs::rename(entry.path(), &to)
            .with_context(|| format!("Moving {} to {}", entry.path().display(), to.display()))?;
    }
    Ok(())
}

/// Based on the current OS, tries to find v1 working directories in the
/// vicinity.
pub(crate) fn find_v1_working_dir(base: impl AsRef<Path>) -> Option<PathBuf> {
    match std::env::consts::OS {
        "windows" => base
            .as_ref()
            .parent()
            .map(|x| x.join("actyxos-data"))
            .filter(|p| p.exists()),
        "android" => None,
        // docker / linux / macos
        _ => base.as_ref().parent().and_then(|parent| {
            // actyxos: ActyxOS on Docker v1
            // actyxos-data: Default for Actyx on Linux
            ["actyxos", "actyxos-data"]
                .iter()
                .map(|x| parent.join(x))
                .find(|p| p.exists())
        }),
    }
}

/// Migrates a given v1 installation, backing up the v1 files within the
/// `v2_working_dir`. If `v1_working_dir` and `v2_working_dir` point to the same
/// directory, the v1 files will be deleted. This assumes operation on a
/// non-existing v2 installation. All generated files will be generated within a
/// temporary directory, and only be moved to the proper place in the end.  If
/// `dry_run` is set, the generated v2 files will not be moved into
/// `v2_working_dir`, nor the v1 files deleted, but the temporary directory
/// persisted and its path printed.
pub fn migrate(
    v1_working_dir: impl AsRef<Path>,
    v2_working_dir: impl AsRef<Path>,
    additional_sources: BTreeSet<SourceId>,
    emit_own_source: bool,
    dry_run: bool,
    version: u32,
) -> anyhow::Result<()> {
    if is_v1_running() {
        bail!("ActyxOS v1 seems to be running. Please stop the process and retry.");
    }

    // assert v1 directory layout
    let v1_dir = assert_v1(&v1_working_dir)?;
    tracing::debug!("V1 dir {} intact", v1_working_dir.as_ref().display());

    // create temporary directory for v2
    let temp_v2 = tempdir_in(&v2_working_dir)?;

    // migrate settings
    migrate_settings(&v1_dir.settings_repo, temp_v2.path())?;

    // Migrate node db
    let mut v2_node_db_conn = copy_db(
        &v1_dir.node_storage.connection.lock(),
        temp_v2.path().join(NODE_DB_FILENAME),
    )?;
    NodeStorage::migrate(&mut v2_node_db_conn, version).context("Migrating settings")?;

    // index db is node db
    // create blocks db
    std::fs::create_dir(temp_v2.path().join("store"))?;
    let v2_blocks_db = temp_v2
        .path()
        .join("store")
        // ipfs-embed block store is named `<topic>.sqlite`
        .join(v1_dir.index_db.with_extension("sqlite").file_name().unwrap());
    let _ = BlockStore::<libipld::DefaultParams>::open(&v2_blocks_db, Default::default())?;
    // migrate swarm dbs
    let v1_source_id: SourceId = {
        let conn = open_readonly(&v1_dir.index_db).context("Opening v1 index db")?;
        conn.query_row("SELECT value FROM meta WHERE key='source'", [], |row| {
            let source_id_text = row.get_ref_unwrap(0).as_str().unwrap();
            let source_id = SourceId::from_str(source_id_text).map_err(|_| rusqlite::Error::QueryReturnedNoRows)?;
            Ok(source_id)
        })
    }
    .context("Getting v1 source id")?;
    let node_id = NodeStorage::query_node_id(&v2_node_db_conn)
        .context("Getting node id")?
        .unwrap();
    let opts = ConversionOptions {
        gc: true,
        vacuum: true,
        filtered_sources: if emit_own_source {
            Some(
                additional_sources
                    .into_iter()
                    .chain(std::iter::once(v1_source_id))
                    .collect(),
            )
        } else {
            Some(additional_sources)
        },
        source_to_stream: maplit::btreemap! { v1_source_id => node_id.stream(0.into()) },
    };
    swarm::convert::convert_from_v1(
        v1_working_dir.as_ref().join("store"),
        temp_v2.path(),
        &*v1_dir.topic,
        "com.actyx.v1-migration",
        opts,
        true,
        NodeStorage::version(&v1_dir.node_storage.connection.lock())?.into(),
        2,
        node_id,
    )
    .context("Converting swarm DBs")?;
    // persist v2 files
    if dry_run {
        let v2_files = temp_v2.into_path();
        tracing::info!(target: "MIGRATION", "Persisted output into {}", v2_files.display());
    } else {
        let backup = backup_v1(&v1_dir, &v2_working_dir).context("Creating v1 backup")?;
        tracing::info!(target:"MIGRATION", "Created backup of v1 files at {}", backup.display());
        if v1_working_dir.as_ref() == v2_working_dir.as_ref() {
            // Remove v1 files in case we're running in the same directory.
            remove_v1(v1_dir).context("Removing v1 files")?;
        } else {
            // Make sure no more file handles are open to any of the files,
            // otherwise the rename operation below fails on Windows.
            drop(v1_dir);
        }
        drop(v2_node_db_conn);
        rename_recursive(temp_v2.path(), &v2_working_dir).context("Moving v2 files")?;
    }

    Ok(())
}
