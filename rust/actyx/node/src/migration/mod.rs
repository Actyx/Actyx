use actyx_sdk::legacy::SourceId;
use rusqlite::OpenFlags;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use crate::{node_storage::NodeStorage, node_storage::CURRENT_VERSION};

pub mod v1;

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
    } else if let Some(wd) = v1::find_v1_working_dir(&base) {
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
pub fn migrate_if_necessary(
    working_dir: impl AsRef<Path>,
    additional_sources: BTreeSet<SourceId>,
    dry_run: bool,
) -> anyhow::Result<()> {
    anyhow::ensure!(working_dir.as_ref().exists());

    let mut additional_sources = Some(additional_sources);
    while let Some((earlier_working_dir, node_db)) = find_earlier_working_dir(&working_dir) {
        // check the db version
        let db_version = get_node_version(&node_db)?;
        match db_version {
            0 | 1 => {
                tracing::info!(target:"MIGRATION",
                    "Migrating data from an earlier version ({} to 2) ..",
                    db_version
                );
                v1::migrate(
                    &earlier_working_dir,
                    &working_dir,
                    additional_sources.take().unwrap_or_default(),
                    true,
                    dry_run,
                    db_version,
                )
                .map_err(|e| {
                    tracing::error!(target: "MIGRATION", "Error during migration: {:#}", e);
                    e
                })?;
                tracing::info!(target:"MIGRATION", "Migration succeeded.");
            }
            2 => {
                tracing::info!(target:"MIGRATION", "Migrating data from an earlier version (2 to 3) ..");
                tracing::debug!("Opening database {}", node_db.display());
                let mut conn = rusqlite::Connection::open(&node_db)?;
                NodeStorage::migrate(&mut conn, db_version)?;
                tracing::info!(target:"MIGRATION", "Migration succeeded.");
            }
            CURRENT_VERSION => break,
            _ => anyhow::bail!("Detected future version {}", db_version),
        }
    }
    Ok(())
}
