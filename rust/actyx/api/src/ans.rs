///! Actyx Naming Service
use actyx_sdk::{app_id, tag, tags, Payload};
use derive_more::{Deref, Display, From};
use futures::{StreamExt, TryFutureExt};
use libipld::cid::Cid;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
use trees::{
    query::{LamportQuery, TagExprQuery, TimeQuery},
    tags::{ScopedTag, ScopedTagSet, TagScope},
};

use crate::BanyanStore;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum PersistenceLevel {
    /// Bits are only resolved on demand, and not protected from garbage collection
    Ephemeral,
    /// Bits are prefetched and aliased right away
    Prefetch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NameRecordEvent {
    Add {
        name: ActyxName,
        // This must not be serialized as a ipld cid!
        #[serde(with = "::actyx_util::serde_str")]
        cid: Cid,
        /// Indicates whether a valid auth token is required to get the files
        public: bool,
        level: PersistenceLevel,
    },
    Remove {
        name: ActyxName,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct NameRecord {
    pub cid: Cid,
    pub level: PersistenceLevel,
    pub public: bool,
}

#[derive(Deref, Display, Clone, Debug, From, Ord, PartialOrd, PartialEq, Eq, Serialize, Deserialize)]
#[from(forward)]
pub struct ActyxName(String);

#[derive(Clone)]
pub struct ActyxNamingService {
    state: Arc<Mutex<BTreeMap<ActyxName, NameRecord>>>,
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
                match serde_cbor::from_slice(event.2.as_slice()) {
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
                            tokio::spawn(
                                store_c
                                    .ipfs()
                                    .clone()
                                    .sync(&cid, store_c.ipfs().peers())
                                    .map_err(move |e| {
                                        tracing::error!(%cid, name=%name_c, error=%e, "Error prefetching");
                                    }),
                            );

                            if let Err(e) = store_c.ipfs().alias(&*name, Some(&cid)) {
                                tracing::error!(%name, error=%e, "Error aliasing");
                            }
                        }

                        state_c.lock().insert(name, NameRecord { cid, level, public });
                    }
                    Ok(NameRecordEvent::Remove { name }) => {
                        tracing::debug!(%name, "Record removal");
                        let _ = store_c.ipfs().alias(&*name, None);
                        state_c.lock().remove(&name);
                    }
                    Err(e) => {
                        tracing::error!(error=%e, "Error decoding ANS record");
                    }
                };
            }
        });
        Self { store, state }
    }

    pub async fn set(
        &self,
        name: impl Into<ActyxName>,
        cid: Cid,
        level: PersistenceLevel,
        public: bool,
    ) -> anyhow::Result<Option<NameRecord>> {
        let name: ActyxName = name.into();
        let record = NameRecordEvent::Add {
            name: name.clone(),
            cid,
            level,
            public,
        };
        self.store
            .append(
                0.into(),
                app_id!("com.actyx"),
                vec![(
                    tags!("ans"),
                    Payload::compact(&record).expect("CBOR Serialization works"),
                )],
            )
            .await?;

        Ok(self.state.lock().insert(name, NameRecord { cid, level, public }))
    }

    pub fn get(&self, name: impl Into<ActyxName>) -> Option<NameRecord> {
        self.state.lock().get(&name.into()).cloned()
    }

    pub async fn remove(&self, name: impl Into<ActyxName>) -> anyhow::Result<Option<NameRecord>> {
        let name: ActyxName = name.into();
        let record = NameRecordEvent::Remove { name: name.clone() };
        self.store
            .append(
                0.into(),
                app_id!("com.actyx"),
                vec![(
                    tags!("ans"),
                    Payload::compact(&record).expect("CBOR Serialization works"),
                )],
            )
            .await?;

        Ok(self.state.lock().remove(&name))
    }
}
