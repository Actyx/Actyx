use std::{convert::TryFrom, path::Path, sync::Arc};

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

#[derive(Clone)]
pub struct NodeStorage {
    pub(crate) connection: Arc<Mutex<Connection>>,
}
impl NodeStorage {
    #[cfg(test)]
    pub fn in_memory() -> Self {
        Self::open(":memory:").expect("Unable to create in memory storage")
    }

    #[cfg(not(test))]
    pub fn new(path_or_name: impl AsRef<Path>) -> anyhow::Result<Self> {
        Self::open(path_or_name)
    }

    fn open(path_or_name: impl AsRef<Path>) -> anyhow::Result<Self> {
        info!("Creating database {}", path_or_name.as_ref().display());
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

    fn initialize_db(conn: &mut Connection) -> anyhow::Result<()> {
        match conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'node'",
            [],
            |row| row.get(0),
        )? {
            0 => {
                conn.execute_batch(
                    "BEGIN;\
                         CREATE TABLE node (name TEXT PRIMARY KEY, value BLOB) WITHOUT ROWID;\
                         INSERT INTO node (name, value) VALUES ('database_version', 2);\
                         COMMIT",
                )?;
            }
            1 => {
                match conn
                    .query_row("SELECT value FROM node WHERE name = 'database_version'", [], |row| {
                        row.get(0)
                    })
                    .optional()?
                {
                    Some(2) => { /* all good */ }
                    Some(1) => return Err(WrongVersionV1.into()),
                    None => return Err(WrongVersionV0.into()),
                    Some(x) => return Err(WrongVersionFuture(x).into()),
                }
            }
            x => bail!("can’t be: {} tables named 'node'", x),
        }

        conn.pragma_update(None, "journal_mode", &"WAL")?;
        conn.pragma_update(None, "synchronous", &"EXTRA")?;

        conn.execute_batch(
            "INSERT INTO node (name, value) VALUES ('cycle_count', -1) ON CONFLICT DO NOTHING;\
                 UPDATE node SET value = value + 1 WHERE name = 'cycle_count'",
        )?;

        Ok(())
    }

    /// version of the node storage. 0 for no version field.
    #[cfg(test)]
    fn version(conn: &Connection) -> anyhow::Result<u32> {
        Ok(conn
            .query_row("SELECT value FROM node WHERE name = 'database_version'", [], |row| {
                row.get(0)
            })
            .optional()
            .map(|x| x.unwrap_or_default())?)
    }

    pub fn set_node_id(&self, key_id: NodeId) -> anyhow::Result<()> {
        let id: PublicKey = key_id.into();
        let id = id.to_string();

        self.connection
            .lock()
            .execute("INSERT INTO node VALUES ('node_id', ?)", &[&id])?;
        Ok(())
    }

    pub fn get_node_key(&self) -> anyhow::Result<Option<NodeId>> {
        if let Some(identity) = self
            .connection
            .lock()
            .query_row("SELECT value FROM node WHERE name='node_id'", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()?
        {
            use std::str::FromStr;
            PublicKey::from_str(&identity).map(Into::into).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn get_keystore(&self) -> anyhow::Result<Option<Box<[u8]>>> {
        if let Some(result) = self
            .connection
            .lock()
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

    pub fn dump_keystore(&self, dump: Box<[u8]>) -> anyhow::Result<()> {
        let encoded = base64::encode(&dump);
        self.connection
            .lock()
            .execute("INSERT OR REPLACE INTO node VALUES ('key_store', ?)", &[&encoded])?;
        Ok(())
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
    use std::time::Duration;

    use rusqlite::backup::Backup;

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

    /// test that we can read a v1 node settings and properly get the node id.
    ///
    /// this is mostly so we have a v1 db in the tests for when we do v2.
    #[test]
    fn should_migrate_v1() -> anyhow::Result<()> {
        let mem = load_test_db("tests/node_v1.sqlite")?;
        assert_eq!(NodeStorage::version(&mem).unwrap(), 1);
        let storage = NodeStorage::from_conn(mem).unwrap();
        let expected_node_id: NodeId = "lBkGGmqD2X/mmtpxnC2KWobZw4g1IWCJSPCdjdB1gCI".parse().unwrap();
        assert_eq!(NodeStorage::version(&storage.connection.lock()).unwrap(), 1);
        assert_eq!(NodeStorage::get_node_key(&storage).unwrap(), Some(expected_node_id));
        Ok(())
    }

    /// Load a sqlite database into a mutable in-memory database
    fn load_test_db(path: &str) -> anyhow::Result<Connection> {
        let mut mem = Connection::open_in_memory()?;
        let v0 = Connection::open(path)?;
        let backup = Backup::new(&v0, &mut mem)?;
        backup.run_to_completion(1000, Duration::from_secs(1), None)?;
        std::mem::drop(backup);
        Ok(mem)
    }
}
