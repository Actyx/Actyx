///! Actyx Naming Service
use actyx_sdk::{app_id, tag, tags, Payload};
use futures::{StreamExt, TryFutureExt};
use libipld::cbor::DagCborCodec;
use libipld::cid::Cid;
use libipld::codec::{Codec, Decode, Encode};
use libipld::DagCbor;
use parking_lot::Mutex;
use std::{collections::BTreeMap, sync::Arc};
use trees::{
    query::{LamportQuery, TagExprQuery, TimeQuery},
    tags::{ScopedTag, ScopedTagSet, TagScope},
};

use crate::BanyanStore;

#[derive(DagCbor, Debug, Clone, Copy)]
pub enum PersistenceLevel {
    /// Bits are only resolved on demand, and not protected from garbage collection
    Ephemeral,
    /// Bits are prefetched and aliased right away
    Prefetch,
}

#[derive(Debug, Clone)]
pub enum NameRecordEvent {
    Add {
        name: String,
        cid: Cid,
        /// Indicates whether a valid auth token is required to get the files
        public: bool,
        level: PersistenceLevel,
    },
    Remove {
        name: String,
    },
}
impl Encode<DagCborCodec> for NameRecordEvent {
    fn encode<W: std::io::Write>(&self, c: DagCborCodec, w: &mut W) -> anyhow::Result<()> {
        let io = match (*self).clone() {
            Self::Add {
                name,
                cid,
                level,
                public,
            } => NameRecordEventIo::Add {
                name,
                public,
                cid: cid.to_string(),
                level,
            },
            Self::Remove { name } => NameRecordEventIo::Remove { name },
        };
        io.encode(c, w)
    }
}
impl Decode<DagCborCodec> for NameRecordEvent {
    fn decode<R: std::io::Read + std::io::Seek>(c: DagCborCodec, r: &mut R) -> anyhow::Result<Self> {
        Ok(match NameRecordEventIo::decode(c, r)? {
            NameRecordEventIo::Add {
                name,
                cid,
                level,
                public,
            } => Self::Add {
                name,
                cid: cid.parse()?,
                public,
                level,
            },
            NameRecordEventIo::Remove { name } => Self::Remove { name },
        })
    }
}
#[derive(DagCbor, Debug)]
enum NameRecordEventIo {
    Add {
        name: String,
        /// Must not use cid, as the referenced data would be pinned recursively with the root maps
        cid: String,
        public: bool,
        level: PersistenceLevel,
    },
    Remove {
        name: String,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct NameRecord {
    pub cid: Cid,
    pub level: PersistenceLevel,
    pub public: bool,
}

#[derive(Clone)]
pub struct ActyxNamingService {
    ingest_handle: Arc<tokio::task::JoinHandle<()>>,
    state: Arc<Mutex<BTreeMap<String, NameRecord>>>,
    store: BanyanStore,
}

fn mk_query() -> TagExprQuery {
    let mut tags: ScopedTagSet = tags!("ans").into();
    tags.insert(ScopedTag::new(TagScope::Internal, tag!("app_id:com.actyx")));
    TagExprQuery::new(vec![tags], LamportQuery::all(), TimeQuery::all())
}

impl ActyxNamingService {
    pub fn new(store: BanyanStore) -> Self {
        let state = Arc::new(Mutex::new(BTreeMap::new()));
        let state_c = state.clone();
        let store_c = store.clone();
        let ingest_handle =
            tokio::spawn(async move {
                let mut stream = store_c.stream_filtered_stream_ordered(mk_query());
                while let Some(event) = stream.next().await {
                    let event = match event {
                        Ok(event) => event,
                        Err(err) => {
                            tracing::warn!(%err, "Error streaming events");
                            continue;
                        }
                    };
                    match DagCborCodec.decode(event.2.as_slice()) {
                        Ok(NameRecordEvent::Add {
                            name,
                            cid,
                            level,
                            public,
                        }) => {
                            tracing::debug!(%name, %cid, "Record Addition");
                            if let PersistenceLevel::Prefetch = level {
                                // Try to sync right away on a best effort basis
                                let name_c = name.clone();
                                tokio::spawn(store_c.ipfs().clone().sync(&cid, store_c.ipfs().peers()).map_err(
                                    move |e| {
                                        tracing::error!(%cid, name=%name_c, error=%e, "Error prefetching");
                                    },
                                ));

                                if let Err(e) = store_c.ipfs().alias(&*name, Some(&cid)) {
                                    tracing::error!(%name, error=%e, "Error aliasing");
                                }
                            }

                            state_c.lock().insert(name, NameRecord { cid, level, public });
                        }
                        Ok(NameRecordEvent::Remove { name }) => {
                            tracing::debug!(%name, "Record removal");
                            let _ = store_c.ipfs().alias(&*name, None);
                            state_c.lock().remove(&*name);
                        }
                        Err(e) => {
                            tracing::error!(error=%e, "Error decoding ANS record");
                        }
                    };
                }
            });
        Self {
            store,
            ingest_handle: Arc::new(ingest_handle),
            state,
        }
    }

    pub async fn set(
        &self,
        name: impl Into<String>,
        cid: Cid,
        level: PersistenceLevel,
        public: bool,
    ) -> anyhow::Result<Option<NameRecord>> {
        let name: String = name.into();
        let record = NameRecordEvent::Add {
            name: name.clone(),
            cid,
            level,
            public,
        };
        let mut buffer = vec![];
        record.encode(DagCborCodec, &mut buffer)?;
        self.store
            .append(
                0.into(),
                app_id!("com.actyx"),
                vec![(tags!("ans"), Payload::from_slice(&buffer))],
            )
            .await?;

        Ok(self.state.lock().insert(name, NameRecord { cid, level, public }))
    }

    pub fn get(&self, name: &str) -> Option<NameRecord> {
        self.state.lock().get(name).cloned()
    }

    pub async fn remove(&self, name: &str) -> anyhow::Result<Option<NameRecord>> {
        let name: String = name.into();
        let record = NameRecordEvent::Remove { name: name.clone() };
        let mut buffer = vec![];
        record.encode(DagCborCodec, &mut buffer)?;
        self.store
            .append(
                0.into(),
                app_id!("com.actyx"),
                vec![(tags!("ans"), Payload::from_slice(&buffer))],
            )
            .await?;

        Ok(self.state.lock().remove(&*name))
    }
}
