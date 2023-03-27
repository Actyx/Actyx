use std::{convert::TryFrom, path::Path, str::FromStr, sync::Arc};

use actyx_sdk::NodeId;
use anyhow::{bail, Context};
use crypto::PublicKey;
use derive_more::{Display, Error};
use parking_lot::Mutex;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use tracing::*;
use util::formats::NodeCycleCount;

#[derive(Debug, Clone, Copy, Display, Error)]
#[display(
    fmt = "Attempting to start Actyx v2.9+ with a data directory from Actyx v2.8 or earlier.\n\
           See the documentation for when and how migration is supported. Meanwhile, you can start from a\n\
           fresh data directory (see also the --working-dir command line option)."
)]
pub struct WrongVersionV2_8;

#[derive(Debug, Clone, Copy, Display, Error)]
#[display(
    fmt = "Attempting to start Actyx v2 with a data directory from ActyxOS v1.1, which is currently not supported.\n\
           See the documentation for when and how migration is supported. Meanwhile, you can start from a\n\
           fresh data directory (see also the --working-dir command line option)."
)]
pub struct WrongVersionV1;

#[derive(Debug, Clone, Copy, Display, Error)]
#[display(
    fmt = "Attempting to start Actyx v2 with a data directory from ActyxOS v1.0, which is currently not supported.\n\
           See the documentation for when and how migration is supported. Meanwhile, you can start from a\n\
           fresh data directory (see also the --working-dir command line option)."
)]
pub struct WrongVersionV0;

#[derive(Debug, Clone, Copy, Display)]
#[display(
    fmt = "Attempting to start Actyx v2 with a data directory from a future version (schema ID is {})",
    _0
)]
pub struct WrongVersionFuture(u32);
impl std::error::Error for WrongVersionFuture {}

pub const CURRENT_VERSION: u32 = 3;

#[derive(Clone)]
pub struct NodeStorage {
    pub(crate) connection: Arc<Mutex<Connection>>,
}
impl NodeStorage {
    pub fn in_memory() -> Self {
        Self::open(":memory:").expect("Unable to create in memory storage")
    }

    pub fn new(path_or_name: impl AsRef<Path>) -> anyhow::Result<Self> {
        Self::open(path_or_name)
    }

    fn open(path_or_name: impl AsRef<Path>) -> anyhow::Result<Self> {
        info!("Using database {}", path_or_name.as_ref().display());
        let flags =
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_FULL_MUTEX;
        let conn = Connection::open_with_flags(path_or_name, flags).context("Opening sqlite for NodeStorage")?;
        Self::from_conn(conn)
    }

    fn from_conn(mut connection: Connection) -> anyhow::Result<Self> {
        Self::initialize_db(&mut connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub fn migrate(conn: &mut Connection, version: u32) -> anyhow::Result<()> {
        match version {
            0 | 1 => Self::migrate_v1(version, conn),
            2 => Ok(conn.execute_batch(
                "DROP TABLE IF EXISTS meta;
                    DROP TABLE IF EXISTS streams;
                    UPDATE node SET value = 3 WHERE name = 'database_version';",
            )?),
            CURRENT_VERSION => Ok(()),
            _ => unreachable!(),
        }
    }

    fn migrate_v1(_version: u32, _conn: &mut Connection) -> anyhow::Result<()> {
        Err(anyhow::anyhow!(
            "migration from ActyxOS v1 was deprecated in version ..., please use version ... to migrate"
        ))
    }

    fn initialize_db(conn: &mut Connection) -> anyhow::Result<()> {
        match conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'node'",
            [],
            |row| row.get(0),
        )? {
            0 => {
                conn.execute_batch(&format!(
                    "BEGIN;\
                         CREATE TABLE node (name TEXT PRIMARY KEY, value BLOB) WITHOUT ROWID;\
                         INSERT INTO node (name, value) VALUES ('database_version', {});\
                         COMMIT",
                    CURRENT_VERSION
                ))?;
            }
            1 => {
                match conn
                    .query_row("SELECT value FROM node WHERE name = 'database_version'", [], |row| {
                        row.get(0)
                    })
                    .optional()?
                {
                    Some(CURRENT_VERSION) => { /* all good */ }
                    Some(2) => return Err(WrongVersionV2_8.into()),
                    Some(1) => return Err(WrongVersionV1.into()),
                    None => return Err(WrongVersionV0.into()),
                    Some(x) => return Err(WrongVersionFuture(x).into()),
                }
            }
            x => bail!("can’t be: {} tables named 'node'", x),
        }

        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "EXTRA")?;

        conn.execute_batch(
            "INSERT INTO node (name, value) VALUES ('cycle_count', -1) ON CONFLICT DO NOTHING;\
                 UPDATE node SET value = value + 1 WHERE name = 'cycle_count'",
        )?;

        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |x| {
            info!(
                "wal_checkpoint(TRUNCATE) returned busy={:?} log={:?} checkpointed={:?}",
                x.get::<_, i64>(0),
                x.get::<_, i64>(1),
                x.get::<_, i64>(2)
            );
            Ok(())
        })?;

        Ok(())
    }

    /// version of the node storage. 0 for no version field.
    pub(crate) fn version(conn: &Connection) -> anyhow::Result<u32> {
        Ok(conn
            .query_row("SELECT value FROM node WHERE name = 'database_version'", [], |row| {
                row.get(0)
            })
            .optional()
            .map(|x| x.unwrap_or_default())?)
    }

    fn persist_node_id(conn: &Connection, node_id: NodeId) -> anyhow::Result<()> {
        let id: PublicKey = node_id.into();
        let id = id.to_string();

        conn.execute("INSERT INTO node VALUES ('node_id', ?)", [&id])?;
        Ok(())
    }

    pub fn set_node_id(&self, node_id: NodeId) -> anyhow::Result<()> {
        Self::persist_node_id(&self.connection.lock(), node_id)
    }

    pub(crate) fn query_node_id(conn: &Connection) -> anyhow::Result<Option<NodeId>> {
        if let Some(identity) = conn
            .query_row("SELECT value FROM node WHERE name='node_id'", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()?
        {
            PublicKey::from_str(&identity).map(Into::into).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn get_node_key(&self) -> anyhow::Result<Option<NodeId>> {
        Self::query_node_id(&self.connection.lock())
    }

    fn query_keystore(conn: &Connection) -> anyhow::Result<Option<Box<[u8]>>> {
        if let Some(result) = conn
            .query_row("SELECT value FROM node WHERE name='key_store'", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()?
        {
            let dump = base64::decode(result)?;
            Ok(Some(dump.into()))
        } else {
            Ok(None)
        }
    }

    pub fn get_keystore(&self) -> anyhow::Result<Option<Box<[u8]>>> {
        Self::query_keystore(&self.connection.lock())
    }

    fn persist_keystore(conn: &Connection, dump: Box<[u8]>) -> anyhow::Result<()> {
        let encoded = base64::encode(&dump);
        conn.execute("INSERT OR REPLACE INTO node VALUES ('key_store', ?)", [&encoded])?;
        Ok(())
    }

    pub fn dump_keystore(&self, dump: Box<[u8]>) -> anyhow::Result<()> {
        Self::persist_keystore(&self.connection.lock(), dump)
    }

    pub fn get_cycle_count(&self) -> anyhow::Result<NodeCycleCount> {
        let cc = self
            .connection
            .lock()
            .query_row("SELECT value FROM node where name = 'cycle_count'", [], |row| {
                row.get::<_, i64>(0)
            })?;
        let res = u64::try_from(cc).map(Into::into)?;
        Ok(res)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_persist_the_node_id() -> anyhow::Result<()> {
        let mut ks = crypto::KeyStore::default();
        let node_id = ks.generate_key_pair().unwrap().into();

        let db = NodeStorage::in_memory();
        db.set_node_id(node_id)?;
        let stored_node_id = db.get_node_key()?.unwrap();

        assert_eq!(node_id, stored_node_id);
        Ok(())
    }
}
