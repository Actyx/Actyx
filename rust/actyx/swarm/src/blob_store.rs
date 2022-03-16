use crate::DbPath;
use actyx_sdk::{AppId, Timestamp};
use derive_more::{Display, Error};
use parking_lot::Mutex;
use rusqlite::{named_params, params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Display, Error)]
#[display(fmt = "stored blob is too large after compression: {}bytes", size)]
pub struct BlobTooLarge {
    #[error(ignore)]
    pub size: usize,
    #[error(ignore)]
    pub limit: usize,
}

#[derive(Clone)]
pub struct BlobStore {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum PathInfo {
    Folder,
    #[serde(rename_all = "camelCase")]
    File {
        original_size: usize,
        compressed_size: usize,
        mime_type: String,
        atime_millis: Option<u64>,
        ctime_millis: u64,
    },
}

impl BlobStore {
    pub fn new(path: DbPath) -> anyhow::Result<Self> {
        let conn = match path {
            DbPath::File(path) => {
                let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
                    | OpenFlags::SQLITE_OPEN_CREATE
                    | OpenFlags::SQLITE_OPEN_FULL_MUTEX;
                Connection::open_with_flags(format!("{}.sqlite", path.display()), flags)?
            }
            DbPath::Memory => Connection::open(":memory:")?,
        };

        conn.execute_batch(
            "\
            PRAGMA journal_mode = TRUNCATE;\n\
            PRAGMA synchronous = NORMAL;\n\
            BEGIN;\n\
            CREATE TABLE IF NOT EXISTS streams \
                (stream TEXT UNIQUE);\n\
            CREATE TABLE IF NOT EXISTS meta \
                (lamport INTEGER);\n\
            CREATE TABLE IF NOT EXISTS blobs \
                (	appId TEXT NOT NULL,\
                    path TEXT NOT NULL,\
                    atime INTEGER,\
                    ctime INTEGER,\
                    size INTEGER,\
                    mimetype TEXT,\
                    compressed BLOB,\
                    PRIMARY KEY (appId, path)\
                );\n\
            COMMIT;",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn blob_put(&self, app_id: AppId, path: String, mime_type: String, data: &[u8]) -> anyhow::Result<()> {
        let _span = tracing::debug_span!("blob_put", appId = %app_id, path = %path).entered();
        let compressed = zstd::encode_all(data, 19)?;
        let size = compressed.len();
        if size > 1048576 {
            return Err(BlobTooLarge { size, limit: 1048576 }.into());
        }
        tracing::trace!(raw = data.len(), compressed = size, "size");

        let mut conn = self.conn.lock();
        let txn = conn.transaction()?;

        // example: path = a/b/c

        // first delete a/b/c/...
        let n = txn
            .prepare_cached("DELETE FROM blobs WHERE (appId = :appId AND substr(path, 1, length(:pathe)) = :pathe)")?
            .execute(named_params! {
                ":appId": app_id.as_str(),
                ":pathe": format!("{}/", path),
            })?;
        tracing::trace!(descendants = n, "deleted");

        // then delete a/b and a
        for (idx, _) in path.rmatch_indices('/') {
            let n = txn
                .prepare_cached("DELETE FROM blobs WHERE appId = ? and path = ?")?
                .execute(params![app_id.as_str(), &path[..idx]])?;
            if n > 0 {
                tracing::trace!(path = &path[..idx], "deleted");
            }
        }

        // the put a/b/c in place, overwriting any previous valud
        let n = txn
            .prepare_cached(
                "INSERT INTO blobs (appId, path, ctime, size, mimetype, compressed) \
                VALUES (:appId, :path, :ctime, :size, :mimetype, :compressed) \
                ON CONFLICT DO UPDATE SET \
                    ctime = :ctime, size = :size, mimetype = :mimetype, compressed = :compressed",
            )?
            .execute(named_params! {
                ":appId": app_id.as_str(),
                ":path": path,
                ":ctime": Timestamp::now().as_i64() / 1000,
                ":size": data.len(),
                ":mimetype": mime_type,
                ":compressed": compressed,
            })?;
        tracing::trace!(rows = n, "stored");

        txn.commit()?;
        Ok(())
    }

    pub fn blob_del(&self, app_id: AppId, path: String) -> anyhow::Result<()> {
        let _span = tracing::debug_span!("blob_del", appId = %app_id, path = %path).entered();
        let mut conn = self.conn.lock();
        let txn = conn.transaction()?;
        let n = txn.prepare_cached(
            "DELETE FROM blobs WHERE (appId = :appId AND (path = :path OR substr(path, 1, length(:pathe)) = :pathe))",
        )?
        .execute(named_params! {
            ":appId": app_id.as_str(),
            ":path": path.as_str(),
            ":pathe": format!("{}/", path),
        })?;
        tracing::trace!(rows = n, "deleted");
        txn.commit()?;
        Ok(())
    }

    pub fn blob_get(&self, app_id: AppId, path: String) -> anyhow::Result<Option<(Vec<u8>, String)>> {
        let _span = tracing::debug_span!("blob_get", appId = %app_id, path = %path).entered();
        let mut conn = self.conn.lock();
        let txn = conn.transaction()?;
        let mut stmt = txn.prepare_cached(
            "SELECT path, atime, ctime, size, length(compressed), mimetype \
                FROM blobs WHERE (appId = :appId AND (path = :path OR substr(path, 1, length(:pathe)) = :pathe))",
        )?;
        let mut res = stmt.query_map(
            named_params! {
                ":appId": app_id.as_str(),
                ":path": path.as_str(),
                ":pathe": format!("{}/", path),
            },
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )?;
        let mut listing = HashMap::new();
        while let Some(row) = res.next() {
            let row = row?;
            if row.0 == path {
                tracing::trace!("found direct match");
                txn.prepare_cached("UPDATE blobs SET atime = ? WHERE appId = ? AND path = ?")?
                    .execute(params![
                        Timestamp::now().as_i64() / 1000,
                        app_id.as_str(),
                        path.as_str()
                    ])?;
                let compressed: Vec<u8> = txn
                    .prepare_cached("SELECT compressed FROM blobs WHERE appId = ? AND path = ?")?
                    .query_row(params![app_id.as_str(), path.as_str()], |row| row.get(0))?;
                drop(res);
                drop(stmt);
                txn.commit()?;
                drop(conn);
                let blob = zstd::decode_all(&*compressed)?;
                return Ok(Some((blob, row.5)));
            } else {
                tracing::trace!(path = %row.0, "found descendant");
                let rest = &row.0[path.len() + 1..];
                match rest.find('/') {
                    Some(idx) => listing.insert(rest[..idx].to_owned(), PathInfo::Folder),
                    None => listing.insert(
                        rest.to_owned(),
                        PathInfo::File {
                            original_size: row.3 as usize,
                            compressed_size: row.4 as usize,
                            mime_type: row.5,
                            atime_millis: row.1.map(|x| x as u64),
                            ctime_millis: row.2 as u64,
                        },
                    ),
                };
            }
        }
        if listing.is_empty() {
            Ok(None)
        } else {
            Ok(Some((serde_json::to_vec(&listing)?, "application/json".to_owned())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;

    #[allow(dead_code)]
    pub fn dbg_row(row: &rusqlite::Row) -> Result<(), rusqlite::Error> {
        let mut s = String::from("row:");
        for idx in 0..row.as_ref().column_count() {
            write!(&mut s, " {:?}", row.get_ref(idx)?).ok();
        }
        println!("{}", s);
        Ok(())
    }

    #[test]
    fn put_get_delete() {
        let store = BlobStore::new(DbPath::Memory).unwrap();
        let app_id = actyx_sdk::app_id!("me");

        assert_eq!(store.blob_get(app_id.clone(), "blob".into()).unwrap(), None);

        store
            .blob_put(app_id.clone(), "blob".into(), "application/xyz".into(), b"abcd")
            .unwrap();
        assert_eq!(
            store.blob_get(app_id.clone(), "blob".into()).unwrap(),
            Some((b"abcd".to_vec(), "application/xyz".to_owned()))
        );

        store.blob_del(app_id.clone(), "blob".into()).unwrap();
        assert_eq!(store.blob_get(app_id, "blob".into()).unwrap(), None);
    }

    #[test]
    fn delete_other() {
        let store = BlobStore::new(DbPath::Memory).unwrap();
        let app_id = actyx_sdk::app_id!("me");
        store
            .blob_put(app_id.clone(), "ab/cd".into(), "application/xyz".into(), b"blob")
            .unwrap();

        store.blob_del(app_id.clone(), "ab/cd/ef".into()).unwrap();
        assert_eq!(
            store.blob_get(app_id.clone(), "ab/cd".into()).unwrap(),
            Some((b"blob".to_vec(), "application/xyz".to_owned()))
        );

        store.blob_del(app_id.clone(), "ab/c".into()).unwrap();
        assert_eq!(
            store.blob_get(app_id.clone(), "ab/cd".into()).unwrap(),
            Some((b"blob".to_vec(), "application/xyz".to_owned()))
        );

        store.blob_del(app_id.clone(), "ab/".into()).unwrap();
        assert_eq!(
            store.blob_get(app_id.clone(), "ab/cd".into()).unwrap(),
            Some((b"blob".to_vec(), "application/xyz".to_owned()))
        );

        store.blob_del(app_id.clone(), "ab".into()).unwrap();
        assert_eq!(store.blob_get(app_id, "ab/cd".into()).unwrap(), None);
    }

    #[test]
    fn put_same() {
        let store = BlobStore::new(DbPath::Memory).unwrap();
        let app_id = actyx_sdk::app_id!("me");
        store
            .blob_put(app_id.clone(), "blob".into(), "application/xyz".into(), b"abcd")
            .unwrap();
        store
            .blob_put(app_id.clone(), "blob".into(), "application/abc".into(), b"ABCD")
            .unwrap();
        assert_eq!(
            store.blob_get(app_id, "blob".into()).unwrap(),
            Some((b"ABCD".to_vec(), "application/abc".to_owned()))
        );
    }

    #[test]
    fn put_parent() {
        let store = BlobStore::new(DbPath::Memory).unwrap();
        let app_id = actyx_sdk::app_id!("me");
        store
            .blob_put(app_id.clone(), "a/b".into(), "application/xyz".into(), b"abcd")
            .unwrap();
        store
            .blob_put(app_id.clone(), "a".into(), "application/xyz".into(), b"ab")
            .unwrap();
        assert_eq!(store.blob_get(app_id, "a/b".into()).unwrap(), None);
    }

    #[test]
    fn put_child() {
        let store = BlobStore::new(DbPath::Memory).unwrap();
        let app_id = actyx_sdk::app_id!("me");
        store
            .blob_put(app_id.clone(), "a/b".into(), "application/xyz".into(), b"abcd")
            .unwrap();
        store
            .blob_put(app_id.clone(), "a/b/c".into(), "application/xyz".into(), b"ab")
            .unwrap();
        assert_eq!(
            store.blob_get(app_id, "a/b".into()).unwrap().unwrap().1,
            "application/json"
        );
    }

    #[test]
    fn put_other() {
        let store = BlobStore::new(DbPath::Memory).unwrap();
        let app_id = actyx_sdk::app_id!("me");
        store
            .blob_put(app_id.clone(), "a/b".into(), "application/xyz".into(), b"abcd")
            .unwrap();
        store
            .blob_put(app_id.clone(), "a/c".into(), "text/plain".into(), b"text")
            .unwrap();
        store
            .blob_put(app_id.clone(), "a/d/e".into(), "text/plain".into(), b"hello")
            .unwrap();
        store
            .blob_put(app_id.clone(), "a/".into(), "text/plain".into(), b"woah")
            .unwrap();

        let (folder, mime) = store.blob_get(app_id.clone(), "a".into()).unwrap().unwrap();
        assert_eq!(mime, "application/json");
        let folder: HashMap<String, PathInfo> = serde_json::from_slice(&*folder).unwrap();

        assert_eq!(folder.len(), 4);
        assert!(matches!(
            &folder["b"],
            PathInfo::File {
                original_size: 4,
                mime_type,
                ..
            } if mime_type == "application/xyz"
        ));
        assert!(matches!(
            &folder["c"],
            PathInfo::File {
                original_size: 4,
                mime_type,
                ..
            } if mime_type == "text/plain"
        ));
        assert!(matches!(&folder["d"], PathInfo::Folder));
        assert!(matches!(
            &folder[""],
            PathInfo::File {
                original_size: 4,
                mime_type,
                ..
            } if mime_type == "text/plain"
        ));

        store
            .blob_put(app_id.clone(), "a//".into(), "text/plain".into(), b"woah")
            .unwrap();
        let folder = store.blob_get(app_id, "a".into()).unwrap().unwrap().0;
        let folder: HashMap<String, PathInfo> = serde_json::from_slice(&*folder).unwrap();
        assert_eq!(folder.len(), 4);
        assert!(matches!(&folder[""], PathInfo::Folder));
    }
}
