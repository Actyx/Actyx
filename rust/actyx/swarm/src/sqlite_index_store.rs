use crate::connectivity::GossipAboutUs;
use actyxos_sdk::{
    event::{LamportTimestamp, SourceId, TimeStamp},
    event_service::snapshots::{
        InvalidateSnapshotsRequest, RetrieveSnapshotRequest, RetrieveSnapshotResponse, StoreSnapshotRequest,
    },
    tagged::{EventKey, StreamId},
    Offset,
};
use anyhow::Result;
use parking_lot::Mutex;
use rusqlite::backup;
use rusqlite::{params, Connection, OpenFlags, OptionalExtension, NO_PARAMS};
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;
use std::{
    collections::{BTreeMap, BTreeSet},
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
        info!("Creating database {:?}", path);
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
                info!("Found lamport = {}", lamport);
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

    pub fn store_snapshot(&mut self, data: StoreSnapshotRequest) -> Result<bool> {
        let mut con = self.conn.lock();
        let tx = con.transaction()?;
        let entity_type = data.entity_type.as_str();
        let name = data.name.as_str();
        let stream = data.key.stream;
        let lamport = data.key.lamport.as_i64();
        let offset = data.key.offset;
        let version = data.version as i64;
        let cycle = data.cycle as i64;
        let tag = data.tag;
        let blob = data.blob;
        let root_offset = serde_cbor::to_vec(&data.offset_map).expect("offset_map should be cborable");
        let horizon = serde_cbor::to_vec(&data.horizon).expect("horizon should be cborable");
        // check if we have snapshots for this entity_type and name with a higher version
        // if that is the case, there is no point in storing the snapshot!
        let higher_count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM snapshots WHERE entity_type=? AND name=? AND version > ?",
            params![entity_type, name, version],
            |row| row.get(0),
        )?;
        if higher_count > 0 {
            // this will abort the transaction
            return Ok(false);
        }
        // delete snapshots with a lower version.
        tx.execute(
            "DELETE FROM snapshots WHERE entity_type=? AND name=? AND version < ?",
            params![entity_type, name, version],
        )?;
        tx.execute(
            r#"
INSERT OR REPLACE INTO snapshots
                   ( entity_type,  name,  version,  lamport,  stream,  offset,  tag,  cycle,  data,  rootoffset,  horizon)
VALUES
      (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
"#,
            params![
                &entity_type,
                &name,
                &version,
                &lamport,
                &stream,
                &offset,
                &tag,
                &cycle,
                &blob,
                &root_offset,
                &horizon
            ],
        )?;
        tx.commit()?;
        Ok(true)
    }

    pub fn invalidate_snapshots(&mut self, data: InvalidateSnapshotsRequest) -> Result<()> {
        let entity_type = data.entity_type.as_str();
        let name = data.name.as_str();
        let stream = data.key.stream;
        let lamport = data.key.lamport.as_i64();
        let offset = data.key.offset;
        self.conn.lock().execute(
            r#"
          DELETE FROM snapshots
          WHERE
            entity_type=?1 AND
            name=?2 AND
            lamport > ?3 OR
              (lamport = ?3 AND stream > ?4) OR
              (lamport = ?3 AND stream = ?4 AND offset >= ?5)
        "#,
            params![&entity_type, &name, &lamport, &stream, &offset,],
        )?;
        Ok(())
    }

    pub fn retrieve_snapshot(&mut self, data: RetrieveSnapshotRequest) -> Result<Option<RetrieveSnapshotResponse>> {
        debug!(
            "Trying to get the snapshot for {}/{}/{}",
            data.name.as_str(),
            data.entity_type.as_str(),
            data.version
        );
        let entity_type = data.entity_type.as_str();
        let name = data.name.as_str();
        let version = data.version as i64;
        struct SnapshotResponse {
            lamport: i64,
            stream_id: StreamId,
            offset: Offset,
            data: String,
            root_offset: Vec<u8>,
            cycle: i64,
            horizon: Vec<u8>,
        }
        // just get the data, no transformation
        let result = self.conn.lock().query_row("SELECT lamport, stream, offset, data, rootoffset, cycle, horizon FROM snapshots WHERE entity_type = ? and name = ? and version = ? order by lamport desc, stream desc, offset desc LIMIT 1",
            params![entity_type, name, version],
            |row|
                Ok(SnapshotResponse {
                    lamport: row.get(0)?,
                    stream_id: row.get(1)?,
                    offset: row.get(2)?,
                    data: row.get(3)?,
                    root_offset: row.get(4)?,
                    cycle: row.get(5)?,
                    horizon: row.get(6)?,
                })
            ,
        ).optional().map_err(|e| {
            warn!("Caught error while trying to get a snapshot: {:?}", e);
            e
        })?;
        // transform the data into the result. This can also fail, so we need all this ceremony
        result
            .map(|r| {
                Ok(RetrieveSnapshotResponse {
                    state: r.data,
                    offset_map: serde_cbor::from_slice(&r.root_offset)?,
                    event_key: EventKey {
                        offset: r.offset,
                        stream: r.stream_id,
                        lamport: LamportTimestamp::new(r.lamport as u64),
                    },
                    horizon: serde_cbor::from_slice(&r.horizon)?,
                    cycle: r.cycle as u64,
                })
            })
            .transpose()
    }

    pub fn invalidate_all_snapshots(&mut self) -> Result<()> {
        self.conn.lock().execute("DELETE FROM snapshots", NO_PARAMS)?;
        Ok(())
    }

    /// current lamport timestamp
    pub fn lamport(&self) -> u64 {
        self.lamport
    }

    #[allow(dead_code)]
    fn write_gossip_about_us(&mut self, gossip_about_us: GossipAboutUs) -> Result<GossipAboutUs> {
        let source_id = gossip_about_us.source_id;
        let offset = gossip_about_us.offset;
        let received_at = gossip_about_us.received_at;
        self.conn
            .lock()
            .prepare_cached("REPLACE INTO gossipaboutus (source, psn, receivedat) VALUES (?1, ?2, ?3)")?
            .execute(params![source_id.as_str(), offset, received_at.as_i64(),])?;
        Ok(GossipAboutUs {
            source_id,
            offset,
            received_at,
        })
    }

    #[allow(dead_code)]
    fn get_all_gossip_about_us(&self) -> Result<BTreeMap<SourceId, GossipAboutUs>> {
        let con = self.conn.lock();
        let mut stmt = con.prepare_cached("SELECT source, psn, receivedat FROM gossipaboutus GROUP BY source")?;
        let gossip_map = stmt
            .query_map(NO_PARAMS, |row| {
                let source: String = row.get(0)?;
                let offset: Offset = row.get(1)?;
                let raw_received_at: i64 = row.get(2)?;

                let source_id = SourceId::from_str(&source).unwrap();
                let received_at = TimeStamp::new(raw_received_at as u64);

                let record = GossipAboutUs {
                    source_id,
                    offset,
                    received_at,
                };

                Ok((source_id, record))
            })?
            .map(|r| r.unwrap())
            .collect();
        Ok(gossip_map)
    }
}

pub fn initialize_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "BEGIN;\n\
        CREATE TABLE IF NOT EXISTS streams \
            (stream TEXT UNIQUE);\n\
        CREATE TABLE IF NOT EXISTS meta \
            (lamport INTEGER);\n\
        CREATE TABLE IF NOT EXISTS snapshots \
            (entity_type TEXT, name TEXT, version INTEGER, lamport INTEGER, stream TEXT, \
            offset INTEGER, tag TEXT, cycle INTEGER, data BLOB, rootoffset BLOB, horizon BLOB, \
            PRIMARY KEY (entity_type, name, version, tag)) WITHOUT ROWID;\n\
        CREATE TABLE IF NOT EXISTS gossipaboutus \
            (source TEXT, psn INTEGER, receivedat INTEGER, PRIMARY KEY (source)) WITHOUT ROWID;\n\
        COMMIT;",
    )?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::connectivity::GossipAboutUs;
    use assert_json_diff::assert_json_eq;
    use quickcheck::{Arbitrary, Gen};
    use rstest::*;
    use serde_json::json;
    use std::str::FromStr;
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
    fn gossip_about_us_should_work(empty_store: SqliteIndexStore) {
        let mut store = empty_store;
        let src = SourceId::from_str("src").unwrap();
        let src2 = SourceId::from_str("src2").unwrap();
        let gossip_about_us = GossipAboutUs {
            source_id: src,
            offset: Offset::mk_test(5),
            received_at: TimeStamp::new(5),
        };
        let gossip_about_us2 = GossipAboutUs {
            source_id: src2,
            offset: Offset::mk_test(7),
            received_at: TimeStamp::new(6),
        };
        store.write_gossip_about_us(gossip_about_us).unwrap();
        store.write_gossip_about_us(gossip_about_us2).unwrap();
        let expected: BTreeMap<SourceId, GossipAboutUs> = vec![(src, gossip_about_us), (src2, gossip_about_us2)]
            .into_iter()
            .collect();
        let retrieved = store.get_all_gossip_about_us().unwrap();
        assert_eq!(retrieved, expected);

        let gossip_about_us3 = GossipAboutUs {
            source_id: src,
            offset: Offset::mk_test(7),
            received_at: TimeStamp::new(6),
        };
        store.write_gossip_about_us(gossip_about_us3).unwrap();
        let expected2: BTreeMap<SourceId, GossipAboutUs> = vec![(src, gossip_about_us3), (src2, gossip_about_us2)]
            .into_iter()
            .collect();
        let retrieved2 = store.get_all_gossip_about_us().unwrap();
        assert_eq!(retrieved2, expected2);
    }

    #[rstest]
    fn backup_test(empty_store: SqliteIndexStore) -> anyhow::Result<()> {
        let mut store = empty_store;
        let src = SourceId::from_str("src").unwrap();
        // write some stuff
        for _ in 0..1000 {
            store.increment_lamport()?;
        }
        let gossip_about_us = GossipAboutUs {
            source_id: src,
            offset: Offset::mk_test(5),
            received_at: TimeStamp::new(5),
        };
        store.write_gossip_about_us(gossip_about_us).unwrap();
        let backed_up = store.backup(DbPath::Memory).unwrap();
        let backed_up_store = SqliteIndexStore::from_conn(Arc::new(Mutex::new(backed_up))).unwrap();
        assert_eq!(backed_up_store.lamport(), store.lamport());
        Ok(())
    }

    #[rstest]
    fn snapshot_happy_1(empty_store: SqliteIndexStore) {
        let mut store = empty_store;
        use serde_json::json;
        let store_request_json = json!({
            "entityType": "foo",
            "name": "bar",
            "key": {
                "stream": "a",
                "offset": 1234,
                "lamport": 0
            },
            "offsetMap": {
                "a": 1,
                "b": 2,
            },
            "horizon": {
                "stream": "a",
                "offset": 1234,
                "lamport": 0
            },
            "cycle": 0,
            "version": 1,
            "tag": "day",
            "blob": "this is the actual snapshot data!",
        });
        let store_request: StoreSnapshotRequest = serde_json::from_value(store_request_json).unwrap();
        let expected_result_json = json!({
            "cycle": 0,
            "horizon": {
                "stream": "a",
                "offset": 1234,
                "lamport": 0
            },
            "eventKey": {
                "stream": "a",
                "offset": 1234,
                "lamport": 0
            },
            "offsetMap": {
                "a": 1,
                "b": 2,
            },
            "state": "this is the actual snapshot data!",
        });
        let retrieve_request_json = json!({
            "entityType": "foo",
            "name": "bar",
            "version": 1,
        });
        let retrieve_request: RetrieveSnapshotRequest = serde_json::from_value(retrieve_request_json).unwrap();

        let invalidate_request_json = json!({
            "entityType": "foo",
            "name": "bar",
            "key": {
                "stream": "a",
                "offset": 1234,
                "lamport": 0
            },
        });
        let invalidate_request: InvalidateSnapshotsRequest = serde_json::from_value(invalidate_request_json).unwrap();

        // store the snapshot
        {
            let store_result = store.store_snapshot(store_request.clone());
            assert_eq!(store_result.unwrap(), true);
        }

        // get the snapshot again - should be there and the same
        {
            let retrieve_result = store.retrieve_snapshot(retrieve_request.clone());
            assert_json_eq!(
                serde_json::to_value(retrieve_result.unwrap()).unwrap(),
                expected_result_json
            );
        }

        // delete all snapshots and get it again - should be gone now!
        {
            store.invalidate_all_snapshots().unwrap();
            let retrieve_result = store.retrieve_snapshot(retrieve_request.clone());
            assert_json_eq!(
                serde_json::to_value(retrieve_result.unwrap()).unwrap(),
                serde_json::Value::Null
            );
        }

        // store the snapshot
        {
            let store_result = store.store_snapshot(store_request);
            assert_eq!(store_result.unwrap(), true);
        }

        // delete all snapshots for this fish and version get it again - should be gone now!
        {
            store.invalidate_snapshots(invalidate_request).unwrap();
            let retrieve_result = store.retrieve_snapshot(retrieve_request);
            assert_json_eq!(
                serde_json::to_value(retrieve_result.unwrap()).unwrap(),
                serde_json::Value::Null
            );
        }
    }

    #[rstest]
    fn snapshot_version_change(empty_store: SqliteIndexStore) {
        let mut store = empty_store;
        use serde_json::json;
        let store_request_new_json = json!({
            "entityType": "foo",
            "name": "bar",
            "key": {
                "stream": "a",
                "offset": 1234,
                "lamport": 0
            },
            "offsetMap": {
                "a": 1,
                "b": 2,
            },
            "horizon": {
                "stream": "a",
                "offset": 1234,
                "lamport": 0
            },
            "cycle": 0,
            "version": 2,
            "tag": "day",
            "blob": "this is a new snapshot",
        });
        let store_request_new: StoreSnapshotRequest = serde_json::from_value(store_request_new_json).unwrap();

        let store_request_old_json = json!({
            "entityType": "foo",
            "name": "bar",
            "key": {
                "stream": "a",
                "offset": 1234,
                "lamport": 0
            },
            "offsetMap": {
                "a": 1,
                "b": 2,
            },
            "horizon": {
                "stream": "a",
                "offset": 1234,
                "lamport": 0
            },
            "cycle": 0,
            "version": 1,
            "tag": "day",
            "blob": "this is an old snapshot",
        });
        let store_request_old: StoreSnapshotRequest = serde_json::from_value(store_request_old_json).unwrap();

        // store the old snapshot - should be stored
        {
            let store_result = store.store_snapshot(store_request_old.clone());
            assert_eq!(store_result.unwrap(), true);
        }

        // store the new snapshot - should be stored
        {
            let store_result = store.store_snapshot(store_request_new);
            assert_eq!(store_result.unwrap(), true);
        }

        // store the old snapshot - storing should be refused
        {
            let store_result = store.store_snapshot(store_request_old);
            assert_eq!(store_result.unwrap(), false);
        }
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
