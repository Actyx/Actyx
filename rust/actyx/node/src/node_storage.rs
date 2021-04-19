use std::sync::Arc;

use actyxos_sdk::NodeId;
use crypto::PublicKey;
use parking_lot::Mutex;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use tracing::*;
use util::formats::{ActyxOSResult, ActyxOSResultExt};

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
    pub fn new<P: AsRef<std::path::Path>>(path_or_name: P) -> ActyxOSResult<Self> {
        Self::open(&path_or_name.as_ref().to_string_lossy())
    }
    pub fn open(path_or_name: &str) -> ActyxOSResult<Self> {
        info!("Creating database {}", path_or_name);
        let flags =
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_FULL_MUTEX;
        let conn = Connection::open_with_flags(path_or_name, flags).ax_internal()?;
        Self::from_conn(conn)
    }

    fn from_conn(mut connection: Connection) -> ActyxOSResult<Self> {
        Self::initialize_db(&mut connection)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    fn initialize_db(conn: &mut Connection) -> ActyxOSResult<()> {
        conn.execute_batch(
            "BEGIN;\n\
            CREATE TABLE IF NOT EXISTS node \
            (name TEXT PRIMARY KEY, value BLOB) WITHOUT ROWID;\n\
            INSERT INTO node VALUES ('database_version', 1) ON CONFLICT DO NOTHING;\n\
            COMMIT;",
        )
        .ax_internal()?;
        conn.execute_batch("PRAGMA journal_mode = WAL;").ax_internal()?;
        // `PRAGMA synchronous = EXTRA;` https://www.sqlite.org/pragma.html#pragma_synchronous
        conn.execute("PRAGMA synchronous = EXTRA;", []).ax_internal()?;

        conn.execute_batch(
           "INSERT INTO node(name,value) SELECT 'cycle_count', -1 WHERE NOT EXISTS (SELECT 1 FROM node WHERE name = 'cycle_count');\n\
                UPDATE node SET value = value+1 WHERE name='cycle_count';").ax_internal()?;

        assert_eq!(NodeStorage::version(conn).ax_internal()?, 1);

        Ok(())
    }

    /// version of the node storage. 0 for no version field.
    fn version(conn: &Connection) -> anyhow::Result<u32> {
        Ok(conn
            .query_row("SELECT value FROM node WHERE name = 'database_version'", [], |row| {
                row.get(0)
            })
            .optional()
            .map(|x| x.unwrap_or_default())?)
    }

    pub fn set_node_id(&self, key_id: NodeId) -> ActyxOSResult<()> {
        let id: PublicKey = key_id.into();
        let id = id.to_string();

        self.connection
            .lock()
            .execute("INSERT INTO node VALUES ('node_id', ?)", &[&id])
            .ax_internal()?;
        Ok(())
    }

    pub fn get_node_key(&self) -> ActyxOSResult<Option<NodeId>> {
        if let Some(identity) = self
            .connection
            .lock()
            .query_row("SELECT value FROM node WHERE name='node_id'", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()
            .ax_internal()?
        {
            use std::str::FromStr;
            PublicKey::from_str(&identity).ax_internal().map(Into::into).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn get_keystore(&self) -> ActyxOSResult<Option<Box<[u8]>>> {
        if let Some(result) = self
            .connection
            .lock()
            .query_row("SELECT value FROM node WHERE name='key_store'", [], |row| {
                row.get::<_, String>(0)
            })
            .optional()
            .ax_internal()?
        {
            let dump = base64::decode(result).ax_internal()?;
            Ok(Some(dump.into()))
        } else {
            Ok(None)
        }
    }

    pub fn dump_keystore(&self, dump: Box<[u8]>) -> ActyxOSResult<()> {
        let encoded = base64::encode(&dump);
        self.connection
            .lock()
            .execute("INSERT OR REPLACE INTO node VALUES ('key_store', ?)", &[&encoded])
            .ax_internal()?;
        Ok(())
    }

    pub fn get_cycle_count(&self) -> ActyxOSResult<u64> {
        self.connection
            .lock()
            .query_row("SELECT value FROM node where name = 'cycle_count'", [], |row| {
                row.get::<_, i64>(0)
            })
            .map(|x| x as u64)
            .ax_internal()
    }
}

#[cfg(test)]
mod test {
    use std::{convert::TryFrom, time::Duration};

    use rusqlite::backup::Backup;

    use super::*;

    #[test]
    fn should_persist_the_node_id() -> ActyxOSResult<()> {
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
        let expected_node_id: NodeId = NodeId::try_from("lBkGGmqD2X/mmtpxnC2KWobZw4g1IWCJSPCdjdB1gCI").unwrap();
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
