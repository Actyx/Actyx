use rusqlite::OpenFlags;
use std::path::{Path, PathBuf};

use crate::{node_storage::NodeStorage, node_storage::CURRENT_VERSION};
use anyhow::Context;

const NODE_DB_FILENAME: &str = "node.sqlite";

fn open_readonly(path: impl AsRef<Path>) -> anyhow::Result<rusqlite::Connection> {
    tracing::debug!("Opening database {}", path.as_ref().display());
    Ok(rusqlite::Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?)
}

fn get_node_version(node_db: impl AsRef<Path>) -> anyhow::Result<u32> {
    NodeStorage::version(&open_readonly(node_db)?)
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

/// Find a working directory of an earlier installation (and the path to the
/// `node.sqlite` database) of Actyx. If the `base` directory is already
/// populated, it will be returned. Otherwise some legacy locations will be
/// tried.  The returned working_directory is guaranteed to exist, but the node
/// db is not.
fn find_earlier_working_dir(base: impl AsRef<Path>) -> Option<(PathBuf, PathBuf)> {
    if base.as_ref().join(NODE_DB_FILENAME).is_file() {
        // first check if `base` points to a populated directory
        Some((base.as_ref().into(), base.as_ref().join(NODE_DB_FILENAME)))
    } else if base.as_ref().join("apps").join(NODE_DB_FILENAME).is_file() {
        // or maybe we're looking at a populated `base` directory running inside
        // ActyxOS on Docker v1?
        Some((base.as_ref().into(), base.as_ref().join("apps").join(NODE_DB_FILENAME)))
    } else if let Some(wd) = find_v1_working_dir(&base) {
        // look for possible v1 candidates in `base.as_ref().parent()`
        if wd.join(NODE_DB_FILENAME).is_file() {
            let db = wd.join(NODE_DB_FILENAME);
            Some((wd, db))
        } else if wd.join("apps").join(NODE_DB_FILENAME).is_file() {
            // or maybe in Docker now?
            let db = wd.join("apps").join(NODE_DB_FILENAME);
            Some((wd, db))
        } else {
            None
        }
    } else {
        // No populated directories found, assuming empty start
        None
    }
}

/// Migrates the Actyx node if necessary. If the node's version is current, this
/// is a no-op.
pub fn migrate_if_necessary(working_dir: impl AsRef<Path>) -> anyhow::Result<()> {
    anyhow::ensure!(working_dir.as_ref().exists());

    while let Some((_, node_db)) = find_earlier_working_dir(&working_dir) {
        // check the db version
        let db_version = get_node_version(&node_db)?;
        match db_version {
            0 | 1 => {
                anyhow::bail!("Migrating from versions 0.x and 1.x is only possible in Actyx versions up to 2.15.0")
            }
            2 => {
                tracing::info!(target:"MIGRATION", "Migrating data from an earlier version (2 to 3) ..");
                tracing::debug!("Opening database {}", node_db.display());
                let mut conn = rusqlite::Connection::open(&node_db)?;
                NodeStorage::migrate(&mut conn, db_version).context("migrating from storage version 2")?;
                tracing::info!(target:"MIGRATION", "Migration succeeded.");
            }
            CURRENT_VERSION => break,
            _ => anyhow::bail!("Detected future version {}", db_version),
        }
    }
    Ok(())
}
