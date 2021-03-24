use crate::error::{LogsvcdError, Result};
use actyxos_sdk::NodeId;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OpenFlags, NO_PARAMS};
use std::convert::TryInto;
use std::default::Default;
use std::ops::Sub;
use std::time::{Duration, UNIX_EPOCH};
use tracing::*;
use util::formats::logs::*;
use util::pinned_resource_sync::PinnedResourceSync;

pub type StorageWrapper = PinnedResourceSync<Storage>;

#[derive(Debug)]
pub struct RetentionStategy {
    max_size: usize,
    older_than_days: chrono::Duration,
}
impl RetentionStategy {
    pub fn new(max_size: usize, older_than_days: usize) -> Self {
        let max_size = max_size;
        let older_than_days = chrono::Duration::days(older_than_days as i64);
        Self {
            max_size,
            older_than_days,
        }
    }
}
impl Default for RetentionStategy {
    fn default() -> Self {
        RetentionStategy {
            max_size: 50_000_000,
            older_than_days: chrono::Duration::days(7),
        }
    }
}

pub struct StorageConfig {
    pub node_name: String,
    pub node_id: NodeId,
    retention: RetentionStategy,
}
impl StorageConfig {
    pub fn new(node_name: String, node_id: NodeId, retention: RetentionStategy) -> Self {
        Self {
            node_name,
            node_id,
            retention,
        }
    }
    pub fn random() -> Self {
        Self {
            node_name: "random really".into(),
            node_id: "uAQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA".try_into().unwrap(),
            retention: RetentionStategy::default(),
        }
    }
}

pub struct Storage {
    conn: Connection,
    config: StorageConfig,
}

impl Storage {
    pub fn in_memory() -> Self {
        Self::open(":memory:", StorageConfig::random()).expect("Unable to create in memory storage")
    }
    pub fn open(path_or_name: &str, config: StorageConfig) -> Result<Self> {
        info!("Creating database {}", path_or_name);
        let flags =
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE | OpenFlags::SQLITE_OPEN_FULL_MUTEX;
        let conn = Connection::open_with_flags(path_or_name, flags)?;
        Self::from_conn(conn, config)
    }
    pub fn change_config<F>(&mut self, f: F)
    where
        F: FnOnce(&mut StorageConfig),
    {
        f(&mut self.config)
    }
    /**
     * Initialize the store from a connection. This is used from open as well as for testing.
     */
    fn from_conn(conn: Connection, config: StorageConfig) -> Result<Self> {
        Self::initialize_db(&conn)?;
        Ok(Self { conn, config })
    }

    fn initialize_db(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "BEGIN;\n\
    CREATE TABLE IF NOT EXISTS meta \
         (key TEXT PRIMARY KEY, value BLOB) WITHOUT ROWID;\n\
	CREATE TABLE IF NOT EXISTS logs \
	     (sequence_number INTEGER PRIMARY KEY AUTOINCREMENT, timestamp INTEGER, log_name TEXT, severity INTEGER, message TEXT,
	     additional_data BLOB, labels BLOB, producer_name TEXT, producer_version TEXT);
        COMMIT;",
        )?;
        // `PRAGMA journal_mode = WAL;` https://www.sqlite.org/wal.html
        // This PRAGMA statement returns the new journal mode, so we need to see if it succeeded
        conn.query_row("PRAGMA journal_mode = WAL;", rusqlite::NO_PARAMS, |row| {
            match row.get_raw(0).as_str().unwrap() {
                "wal" => Ok("wal"),
                "memory" => Ok("memory"), // There is no WAL for memory databases TODO Rust error handling
                _other => Err(rusqlite::Error::InvalidQuery),
            }
        })?;
        // `PRAGMA synchronous = NORMAL;` https://www.sqlite.org/pragma.html#pragma_synchronous
        conn.execute("PRAGMA synchronous = NORMAL;", rusqlite::NO_PARAMS)?;
        Ok(())
    }

    pub fn add_logs(&mut self, logs: Vec<LogRequest>) -> Result<()> {
        let tx = self.conn.transaction()?;
        trace!("About to insert {} logs.", logs.len());
        {
            let mut stmt = tx.prepare(
                "INSERT INTO logs (timestamp, log_name, severity, message, additional_data, labels, producer_name, producer_version) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for log in logs {
                stmt.insert(params![
                    log.log_timestamp.unwrap_or_else(Utc::now).timestamp_millis(),
                    log.log_name,
                    log.severity.to_level(),
                    log.message,
                    serde_cbor::to_vec(&log.additional_data)?,
                    serde_cbor::to_vec(&log.labels)?,
                    log.producer_name,
                    log.producer_version
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn install_hook(&self, queue: crossbeam::channel::Sender<i64>) {
        let closure: Box<dyn FnMut(rusqlite::Action, &str, &str, i64) + Send + 'static> =
            Box::new(move |action, db_name, table, row_id| {
                trace!("{:?} {} {} {}", action, db_name, table, row_id);
                if action == rusqlite::Action::SQLITE_INSERT && table == "logs" {
                    // We must not block here. If `queue` is full, we're deadlocked.
                    let _ = queue.try_send(row_id);
                }
            });
        self.conn.update_hook(Some(closure))
    }

    fn parse_log(&self, row: &rusqlite::Row) -> rusqlite::Result<LogEvent> {
        let d = UNIX_EPOCH + Duration::from_millis(row.get::<_, i64>(0)? as u64);
        let log_timestamp = DateTime::<Utc>::from(d);
        let log = LogEvent {
            node_name: self.config.node_name.clone(),
            node_id: self.config.node_id,
            log_timestamp,
            log_name: row.get(1)?,
            severity: LogSeverity::from_level(row.get(2)?),
            message: row.get(3)?,
            additional_data: serde_cbor::from_slice(row.get_raw(4).as_blob()?).unwrap(),
            labels: serde_cbor::from_slice(row.get_raw(5).as_blob()?).unwrap(),
            sequence_number: row.get::<_, i64>(6)? as u64,
            producer_name: row.get(7)?,
            producer_version: row.get(8)?,
        };
        Ok(log)
    }
    pub fn get_highest_seq(&self, timestamp: Option<DateTime<Utc>>) -> Result<i64> {
        self.conn
            .query_row(
                "SELECT MAX(sequence_number) from logs WHERE timestamp <= ?1",
                &[timestamp.map(|x| x.timestamp_millis()).unwrap_or_else(|| std::i64::MAX)],
                |row| row.get(0).or(Ok(-1)),
            )
            .map_err(LogsvcdError::RusqliteError)
    }
    pub fn get_logs_by_seq(
        &self,
        from: i64,
        to: Option<i64>, // None means max
    ) -> Result<Vec<LogEvent>> {
        Ok(self.conn.prepare(
            "SELECT timestamp, log_name, severity, message, additional_data, labels, sequence_number, producer_name, producer_version FROM logs WHERE sequence_number >= ?1 and sequence_number <= ?2",
        )?
                    .query_map(
                        params![from, to.unwrap_or_else(|| std::isize::MAX.try_into().unwrap())],
                        |r| self.parse_log(r),
                    )?
                    .filter_map(|x| x.ok())
                    .collect())
    }
    #[allow(clippy::let_and_return)]
    #[cfg(test)] // might be used at some point
    pub fn get_logs(
        &self,
        from: DateTime<Utc>,
        to: Option<DateTime<Utc>>, // defaults to now
        log_name: Option<String>,
        severity: Option<LogSeverity>,
    ) -> Result<Vec<LogEvent>> {
        debug!(
            "Querying logs from {} to {} (log_name: {}, severity: {:?} ({}))",
            from.to_rfc3339(),
            to.unwrap_or_else(Utc::now).to_rfc3339(),
            log_name.clone().unwrap_or_else(|| "*".to_string()),
            severity.clone().unwrap_or_default(),
            severity.clone().unwrap_or_default().to_level()
        );
        let logs = if let Some(log_name) = log_name {
            let mut stmt = self.conn.prepare(
            "SELECT timestamp, log_name, severity, message, additional_data, labels, sequence_number, producer_name, producer_version FROM logs WHERE timestamp >= ?1 and timestamp <= ?2 and log_name = ?3 and severity >= ?4",
        )?;
            let x = stmt
                .query_map(
                    params![
                        from.timestamp_millis(),
                        to.unwrap_or_else(Utc::now).timestamp_millis(),
                        log_name,
                        severity.unwrap_or_default().to_level()
                    ],
                    |r| self.parse_log(r),
                )?
                .filter_map(|x| x.ok())
                .collect::<Vec<_>>();
            x
        } else {
            let mut stmt = self.conn.prepare(
            "SELECT timestamp, log_name, severity, message, additional_data, labels, sequence_number, producer_name, producer_version FROM logs WHERE timestamp >= ?1 and timestamp <= ?2 and severity >= ?3",
        )?;
            let x = stmt
                .query_map(
                    params![
                        from.timestamp_millis(),
                        to.unwrap_or_else(Utc::now).timestamp_millis(),
                        severity.unwrap_or_default().to_level()
                    ],
                    |r| self.parse_log(r),
                )?
                .filter_map(|x| x.ok())
                .collect::<Vec<_>>();
            x
        };
        debug!("Got {} logs.", logs.len());
        Ok(logs)
    }

    // Gets the current db size in bytes
    fn get_db_size(&self) -> Result<usize> {
        self.conn
            // https://stackoverflow.com/a/52191503
            .query_row(
                "SELECT page_count * page_size as size FROM pragma_page_count(), pragma_page_size()",
                rusqlite::NO_PARAMS,
                |r| r.get(0).map(|x: i64| x.try_into().unwrap()),
            )
            .map_err(LogsvcdError::RusqliteError)
    }

    fn prune_logs(&self, older_than: DateTime<Utc>) -> Result<usize> {
        let rows = self
            .conn
            .execute("DELETE FROM logs WHERE timestamp < ?", &[older_than.timestamp_millis()])
            .map_err(LogsvcdError::RusqliteError)?;

        self.conn
            .execute("VACUUM", NO_PARAMS)
            .map_err(LogsvcdError::RusqliteError)?;

        Ok(rows)
    }

    // TODO: think about fairness per log level
    #[allow(clippy::cognitive_complexity)]
    pub fn prune(&self) -> Result<()> {
        let size_before = self.get_db_size()?;
        debug!("Pruning the database");

        self.prune_logs(Utc::now().sub(self.config.retention.older_than_days))?;
        let mut keep = self.config.retention.older_than_days;
        // keep on pruning until data usage is <= 70 % of max_size
        while 10 * self.get_db_size()? >= 7 * self.config.retention.max_size {
            keep = keep - chrono::Duration::hours(1);
            if keep.num_hours() > 0 {
                info!("DB needs further pruning, keeping last {} days.", keep.num_hours() / 24);
                self.prune_logs(Utc::now().sub(keep))?;
            } else {
                error!(
                    "Logs generated in the last hour amounts to {} bytes. Retention size is set to {}. Pruning failed. DB has been truncated completely.",
                    self.get_db_size()?,
                    self.config.retention.max_size
                );
                self.prune_logs(Utc::now())?;
                break;
            }
        }
        let size_after = self.get_db_size()?;
        info!(
            "DB pruned from {} MB to {} MB.",
            size_before as f64 / 1e6,
            size_after as f64 / 1e6
        );

        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use maplit::btreemap;

    pub(crate) fn insert_dummy_logs(storage: &mut Storage, cnt: i64) {
        let log = LogRequest {
            log_timestamp: None,
            severity: LogSeverity::Info,
            message: "".into(),
            log_name: "log_name".into(),
            additional_data: None,
            labels: Some(btreemap! {
            "origin".into() => "mars".into()
            }),
            producer_name: "someone".into(),
            producer_version: "-42".into(),
        };
        let logs = (0..cnt)
            .map(|idx| {
                let mut req = log.clone();
                req.log_timestamp = Some(Utc::now());
                req.message = format!("Message {}", idx);
                req
            })
            .collect();
        storage.add_logs(logs).unwrap();
        assert_eq!(cnt, storage.get_highest_seq(None).unwrap());
    }

    #[test]
    fn smoke() {
        let mut storage = Storage::in_memory();
        insert_dummy_logs(&mut storage, 100);
    }

    #[test]
    fn should_be_able_to_insert_and_get_logs() {
        let mut storage = Storage::in_memory();
        let now = Utc::now();
        let labels = Some(btreemap! {
        "origin".to_owned() => "mars".to_owned()
        });
        let log_name = "log_name".to_owned();
        let additional_data = None;

        let log = LogRequest {
            log_timestamp: Some(now),
            severity: LogSeverity::Info,
            message: "message".to_owned(),
            log_name: log_name.clone(),
            additional_data: additional_data.clone(),
            labels: labels.clone(),
            producer_name: "someone".to_string(),
            producer_version: "-42".to_string(),
        };
        storage.add_logs(vec![log]).unwrap();
        let ret = &storage
            .get_logs(now, None, Some(log_name), Some(LogSeverity::Trace))
            .unwrap()[0];
        println!("{:?}", ret);
        assert_eq!(ret.sequence_number, 1);
        assert_eq!(ret.additional_data, additional_data);
        assert_eq!(ret.labels, labels);
        assert_eq!(ret.log_timestamp.timestamp_millis(), now.timestamp_millis());
    }
}
