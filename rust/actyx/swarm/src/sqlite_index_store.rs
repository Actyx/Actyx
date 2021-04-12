use actyxos_sdk::StreamId;
use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::backup;
use rusqlite::{params, Connection, OpenFlags, NO_PARAMS};
use std::path::PathBuf;
use std::time::Duration;
use std::{
    collections::BTreeSet,
    sync::Arc,
};
use tracing::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DbPath {
    File(PathBuf),
    Memory,
}

pub struct SqliteIndexStore {
    conn: Arc<Mutex<Connection>>,
    /// local copy of the lamport timestamp for quick access
    /// This must be ensured to be always in sync with the db value
    ///
    /// We do not use the newtype here, since we need to perform operations with it
    /// that are not supported by the newtype.
    lamport: u64,
}

/// Implementation of IpfsIndexStore for sqlite. Please note that for this implementation
/// offsets are converted from u64 to i64 for transfer to the database. So please use only
/// lower half! (should not be a problem for JS, which uses 53 bits anyway)
impl SqliteIndexStore {
    #[allow(dead_code)]
    pub fn backup(&self, path: DbPath) -> Result<Connection> {
        info!("Creating backup database {:?}", path);
        let flags =
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_FULL_MUTEX;
        let mut target = match path {
            DbPath::File(path) => Connection::open_with_flags(path, flags),
            DbPath::Memory => Connection::open(":memory:"),
        }?;
        {
            let con = self.conn.lock();
            let backup = backup::Backup::new(&con, &mut target)?;
            let progress = |x: rusqlite::backup::Progress| {
                info!("backup progress {} / {}", x.pagecount - x.remaining, x.pagecount);
            };
            backup.run_to_completion(1000, Duration::from_millis(250), Some(progress))?;
        }
        Ok(target)
    }

    // #[instrument(level = "debug")]
    pub fn open(path: DbPath) -> Result<Self> {
        debug!("Creating database {:?}", path);
        // These are the same flags that the npm SQLite package has
        let flags =
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_FULL_MUTEX;
        let conn = match path {
            DbPath::File(path) => Connection::open_with_flags(format!("{}.sqlite", path.display()), flags),
            DbPath::Memory => Connection::open(":memory:"),
        }?;
        Self::from_conn(Arc::new(Mutex::new(conn)))
    }

    /**
     * Initialize the store from a connection. This is used from `open` as well
     * as for testing.
     **/
    pub fn from_conn(conn: Arc<Mutex<Connection>>) -> Result<Self> {
        let locked = conn.lock();
        initialize_db(&locked)?;
        let lamport = locked
            .query_row("SELECT lamport FROM meta", NO_PARAMS, |row| {
                let lamport: i64 = row.get(0)?;
                let lamport = lamport as u64;
                debug!("Found lamport = {}", lamport);
                Ok(lamport)
            })
            .or_else(|_| -> Result<u64> {
                locked.execute("INSERT INTO meta VALUES (0)", NO_PARAMS)?;
                Ok(0)
            })?;
        drop(locked);
        Ok(Self { conn, lamport })
    }

    /// we received a lamport from an external source
    pub fn received_lamport(&mut self, lamport: u64) -> Result<u64> {
        let lamport = lamport + 1;
        self.conn
            .lock()
            .prepare_cached("UPDATE meta SET lamport = MAX(lamport, ?)")?
            .execute(params![&(lamport as i64)])?;
        // do this after the txn was successfully committed.
        self.lamport = self.lamport.max(lamport);
        trace!("received lamport {}, current lamport is {}", lamport, self.lamport);
        Ok(self.lamport)
    }

    /// Increment the lamport and return the new value
    pub fn increment_lamport(&mut self) -> Result<u64> {
        self.conn
            .lock()
            .prepare_cached("UPDATE meta SET lamport = lamport + 1")?
            .execute(NO_PARAMS)?;
        // do this after the txn was successfully committed.
        self.lamport += 1;
        trace!("incremented lamport to {}", self.lamport);
        Ok(self.lamport)
    }

    /// Increase the lamport by `increment` and return the new value
    pub fn increase_lamport(&mut self, increment: u32) -> Result<u64> {
        self.conn
            .lock()
            .prepare_cached("UPDATE meta SET lamport = lamport + ?")?
            .execute(params![&(increment as i64)])?;
        // do this after the txn was successfully committed.
        self.lamport += increment as u64;
        trace!("increased lamport by {} to {}", increment, self.lamport);
        Ok(self.lamport)
    }

    pub fn add_stream(&mut self, stream: StreamId) -> Result<()> {
        let result = self
            .conn
            .lock()
            .prepare_cached("INSERT INTO streams VALUES(?)")?
            .execute(params![&stream]);
        match result {
            Ok(_)
            // Violation of unique constraint --> StreamId is already present
            | Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error {
                    code: rusqlite::ffi::ErrorCode::ConstraintViolation,
                    extended_code: 2067,
                },
                ..,
            )) => Ok(()),
            Err(err) => Err(err),
        }?;
        Ok(())
    }
    pub fn get_observed_streams(&mut self) -> Result<BTreeSet<StreamId>> {
        let con = self.conn.lock();
        let mut stmt = con.prepare("SELECT * from streams")?;
        let result = stmt.query_map(NO_PARAMS, |r| {
            let stream_id: StreamId = r.get(0)?;
            Ok(stream_id)
        })?;

        let mut set: BTreeSet<StreamId> = Default::default();
        for s in result {
            set.insert(s?);
        }
        Ok(set)
    }


    /// current lamport timestamp
    pub fn lamport(&self) -> u64 {
        self.lamport
    }
}

pub fn initialize_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;\n\
        CREATE TABLE IF NOT EXISTS streams \
            (stream TEXT UNIQUE);\n\
        CREATE TABLE IF NOT EXISTS meta \
            (lamport INTEGER);\n\
        COMMIT;",
    )?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::{Arbitrary, Gen};
    use rstest::*;
    use tempdir::TempDir;

    fn get_shared_memory_index_store(path: &str) -> Result<SqliteIndexStore> {
        SqliteIndexStore::open(DbPath::File(path.into()))
    }

    #[fixture]
    fn empty_store() -> SqliteIndexStore {
        SqliteIndexStore::open(DbPath::Memory).unwrap()
    }

    #[rstest]
    fn received_lamport_should_take_the_max_and_increment(mut empty_store: SqliteIndexStore) {
        empty_store.received_lamport(5).unwrap();
        assert_eq!(empty_store.lamport, 6);
        empty_store.received_lamport(3).unwrap();
        assert_eq!(empty_store.lamport, 6);
    }

    #[rstest]
    fn creating_a_new_store_should_grab_lamport_from_the_db() -> anyhow::Result<()> {
        let dir = TempDir::new("grab_lamport_from_db").expect("cannot create TempDir");
        let db = dir.path().join("db").to_str().expect("illegal filename").to_owned();
        let mut store = get_shared_memory_index_store(&db)?;
        for _ in 0..4 {
            store.increment_lamport()?;
        }
        let other_store = get_shared_memory_index_store(&db)?;

        assert_eq!(store.lamport, other_store.lamport);
        Ok(())
    }

    #[rstest]
    fn backup_test(empty_store: SqliteIndexStore) -> anyhow::Result<()> {
        let mut store = empty_store;
        // write some stuff
        for _ in 0..1000 {
            store.increment_lamport()?;
        }
        let backed_up = store.backup(DbPath::Memory).unwrap();
        let backed_up_store = SqliteIndexStore::from_conn(Arc::new(Mutex::new(backed_up))).unwrap();
        assert_eq!(backed_up_store.lamport(), store.lamport());
        Ok(())
    }

    #[rstest]
    fn stream_id_persistence(empty_store: SqliteIndexStore) {
        let mut s = empty_store;
        let mut g = Gen::new(42);
        let streams: BTreeSet<StreamId> = Arbitrary::arbitrary(&mut g);

        for i in &streams {
            s.add_stream(*i).unwrap();
            // check dedup
            s.add_stream(*i).unwrap();
        }

        let received = s.get_observed_streams().unwrap();
        assert_eq!(received, streams);
    }
}
