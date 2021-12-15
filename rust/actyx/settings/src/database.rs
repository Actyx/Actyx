use std::path::Path;

use rusqlite::{params, Connection, OpenFlags, OptionalExtension, Transaction as RusqlTransaction};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Error: {0}")]
    DbError(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Database {
    conn: Connection,
}
pub struct Transaction<'a> {
    tx: RusqlTransaction<'a>,
}

pub const DB_FILENAME: &str = "settings.db";

impl Database {
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        std::fs::create_dir_all(base_dir.as_ref()).map_err(|err| Error::IoError(format!("{}", err)))?;
        let flags =
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_FULL_MUTEX;
        let conn = Connection::open_with_flags(base_dir.as_ref().join(DB_FILENAME), flags)?;
        Self::initialize(&conn)?;
        Ok(Database { conn })
    }
    pub fn from_connection_without_init(conn: Connection) -> Self {
        Self { conn }
    }
    pub fn in_memory() -> Result<Self> {
        let flags =
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_FULL_MUTEX;
        let conn = Connection::open_with_flags(":memory:", flags)?;
        Self::initialize(&conn)?;
        Ok(Database { conn })
    }
    fn initialize(conn: &Connection) -> Result<()> {
        // We can't use the timestamp as a primary key for the settings table, as sqlite only gives
        // us precision up to ms: https://www.sqlite.org/datatype3.html
        conn.execute_batch(
            "BEGIN;\n\
             CREATE TABLE IF NOT EXISTS schemas \
             (scope TEXT PRIMARY KEY, schema TEXT) WITHOUT ROWID;\n\
             CREATE TABLE IF NOT EXISTS settings \
             (id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP, settings TEXT);\n\
             CREATE TABLE IF NOT EXISTS valid_settings_with_defaults \
             (id INTEGER PRIMARY KEY, settings TEXT) WITHOUT ROWID;\n\
             COMMIT;",
        )?;
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
        // `PRAGMA synchronous = EXTRA;` https://www.sqlite.org/pragma.html#pragma_synchronous
        conn.execute("PRAGMA synchronous = EXTRA;", [])?;
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE);", [], |x| {
            tracing::info!(
                "wal_checkpoint(TRUNCATE) returned busy={:?} log={:?} checkpointed={:?}",
                x.get::<_, i64>(0),
                x.get::<_, i64>(1),
                x.get::<_, i64>(2)
            );
            Ok(())
        })?;
        Ok(())
    }
    pub fn exec<R>(&mut self, update: impl FnOnce(&mut Transaction) -> R) -> Result<R> {
        let tx = self.conn.transaction()?;
        let mut dt = Transaction { tx };
        let result = update(&mut dt);
        dt.tx.commit()?;
        Ok(result)
    }
}

impl<'a> Transaction<'a> {
    pub fn set_schema(&mut self, scope: String, schema: String) -> Result<()> {
        let _ = self.tx.execute(
            "INSERT OR REPLACE INTO schemas (scope, schema) VALUES (?,?)",
            params![scope, schema],
        )?;
        Ok(())
    }

    /// Returns a schema for a given scope, if any.
    pub fn get_schema(&mut self, scope: String) -> Result<Option<String>> {
        let res = self
            .tx
            .query_row("SELECT schema FROM schemas WHERE scope=?", params![scope], |row| {
                row.get(0)
            })
            .optional()?;
        Ok(res)
    }

    /// Returns all installed schemas with their respective scopes.
    pub fn get_all_schema_scopes(&mut self) -> Result<Vec<String>> {
        let mut stmt = self.tx.prepare("SELECT scope FROM schemas")?;
        let res: Vec<String> = stmt
            .query_map(params![], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(res)
    }

    /// Unconditionally deletes a schema.
    pub fn delete_schema(&mut self, scope: String) -> Result<bool> {
        let c = self.tx.execute("DELETE FROM schemas WHERE scope=?", params![scope])?;
        Ok(c > 0)
    }

    /// Returns the root settings object if any.
    pub fn get_settings(&mut self) -> Result<Option<String>> {
        let res = self
            .tx
            .query_row(
                "SELECT settings FROM settings ORDER BY id DESC LIMIT 1",
                params![],
                |row| row.get(0),
            )
            .optional()?;
        Ok(res)
    }
    pub fn set_settings(&mut self, settings: String) -> Result<()> {
        let _ = self
            .tx
            .execute("INSERT INTO settings (settings) VALUES (?)", params![settings])?;
        Ok(())
    }
}
