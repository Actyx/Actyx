use actyx_sdk::{LamportTimestamp, StreamId};
use anyhow::{Context, Result};
use ax_futures_util::stream::variable::{Observer, Variable};
use parking_lot::Mutex;
use rusqlite::backup;
use rusqlite::{params, Connection, OpenFlags};
use std::time::Duration;
use std::{collections::BTreeSet, sync::Arc};
use std::{convert::TryFrom, path::PathBuf};
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
    lamport: Variable<LamportTimestamp>,
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
        }
        .context("Open sqlite for SqliteIndexStore")?;
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
            .query_row("SELECT lamport FROM meta", [], |row| {
                let lamport: i64 = row.get(0)?;
                let lamport = lamport as u64;
                debug!("Found lamport = {}", lamport);
                Ok(lamport)
            })
            .or_else(|_| -> Result<u64> {
                locked.execute("INSERT INTO meta VALUES (0)", [])?;
                Ok(0)
            })?;
        drop(locked);
        Ok(Self {
            conn,
            lamport: Variable::new(lamport.into()),
        })
    }

    /// we received a lamport from an external source
    pub fn received_lamport(&mut self, lamport: LamportTimestamp) -> Result<()> {
        let conn = self.conn.lock();
        let res: i64 = conn
            .prepare_cached("UPDATE meta SET lamport = MAX(lamport + 1, ?) RETURNING lamport")?
            .query_row(params![u64::from(lamport) as i64], |x| x.get(0))?;
        self.lamport.set(u64::try_from(res).expect("negative lamport").into());
        drop(conn);
        Ok(())
    }

    /// Increase the lamport by `increment` and return the *initial* value
    pub fn increase_lamport(&mut self, increment: u64) -> Result<LamportTimestamp> {
        let conn = self.conn.lock();
        let res: i64 = conn
            .prepare_cached("UPDATE meta SET lamport = lamport + ? RETURNING lamport")?
            .query_row(params![&(increment as i64)], |x| x.get(0))?;
        let initial = self.lamport.get();
        self.lamport.set(u64::try_from(res).expect("negative lamport").into());
        drop(conn);
        trace!("increased lamport by {}", increment);
        Ok(initial)
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
        let result = stmt.query_map([], |r| {
            let stream_id: StreamId = r.get(0)?;
            Ok(stream_id)
        })?;

        let mut set: BTreeSet<StreamId> = Default::default();
        for s in result {
            set.insert(s?);
        }
        Ok(set)
    }

    pub fn observe_lamport(&self) -> Observer<LamportTimestamp> {
        self.lamport.new_observer()
    }

    /// current lamport timestamp, for testing
    #[cfg(test)]
    pub fn lamport(&self) -> actyx_sdk::LamportTimestamp {
        self.lamport.get()
    }
}

pub fn initialize_db(conn: &Connection) -> Result<()> {
    // `PRAGMA journal_mode = WAL;` https://www.sqlite.org/wal.html
    // This PRAGMA statement returns the new journal mode, so we need to see if it succeeded
    conn.query_row("PRAGMA journal_mode = WAL;", [], |row| {
        let res: String = row.get(0)?;
        match res.as_str() {
            "wal" => Ok("wal"),
            "memory" => Ok("memory"), // There is no WAL for memory databases
            _other => Err(rusqlite::Error::InvalidQuery),
        }
    })?;
    // `PRAGMA synchronous = NORMAL;` https://www.sqlite.org/pragma.html#pragma_synchronous
    conn.execute("PRAGMA synchronous = NORMAL;", [])?;
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

    fn get_shared_memory_index_store(path: &str) -> Result<SqliteIndexStore> {
        SqliteIndexStore::open(DbPath::File(path.into()))
    }

    fn empty_store() -> SqliteIndexStore {
        SqliteIndexStore::open(DbPath::Memory).unwrap()
    }

    #[test]
    fn received_lamport_should_take_the_max_and_increment() {
        let mut empty_store = empty_store();
        empty_store.received_lamport(5.into()).unwrap();
        assert_eq!(empty_store.lamport.get(), LamportTimestamp::from(5));
        empty_store.received_lamport(3.into()).unwrap();
        assert_eq!(empty_store.lamport.get(), LamportTimestamp::from(6));
    }

    #[test]
    fn creating_a_new_store_should_grab_lamport_from_the_db() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let db = dir.path().join("db").to_str().expect("illegal filename").to_owned();
        let mut store = get_shared_memory_index_store(&db)?;
        for _ in 0..4 {
            store.increase_lamport(1)?;
        }
        let other_store = get_shared_memory_index_store(&db)?;

        assert_eq!(store.lamport.get(), other_store.lamport.get());
        Ok(())
    }

    #[test]
    fn backup_test() -> Result<()> {
        let mut store = empty_store();
        // write some stuff
        for _ in 0..1000 {
            store.increase_lamport(1)?;
        }
        let backed_up = store.backup(DbPath::Memory).unwrap();
        let backed_up_store = SqliteIndexStore::from_conn(Arc::new(Mutex::new(backed_up))).unwrap();
        assert_eq!(backed_up_store.lamport(), store.lamport());
        Ok(())
    }

    #[test]
    fn stream_id_persistence() {
        let mut s = empty_store();
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
