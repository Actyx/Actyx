use std::{collections::BTreeMap, sync::Arc};

use actyx_sdk::{app_id, tag, tags, Payload};
use futures::StreamExt;
use libipld::cbor::DagCborCodec;
use libipld::cid::Cid;
use libipld::codec::{Codec,  Encode};
use libipld::DagCbor;
use parking_lot::Mutex;
use trees::{
    query::{LamportQuery, TagExprQuery, TimeQuery},
    tags::{ScopedTag, ScopedTagSet, TagScope},
};

use crate::BanyanStore;

///! Actyx Naming Service

// TODO: Add added removed etc
#[derive(DagCbor, Debug)]
pub struct NameRecord {
    name: String,
    cid: Cid,
}

#[derive(Clone)]
pub struct ActyxNamingService {
    ingest_handle: Arc<tokio::task::JoinHandle<()>>,
    state: Arc<Mutex<BTreeMap<String, Cid>>>,
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
        let mut stream = store.stream_filtered_stream_ordered(mk_query());
        let ingest_handle = tokio::spawn(async move {
            while let Some(event) = stream.next().await {
                let event = match event {
                    Ok(event) => event,
                    Err(err) => {
                        tracing::warn!("{}", err);
                        continue;
                    }
                };
                match DagCborCodec.decode(event.2.as_slice()) {
                    Ok(NameRecord { name, cid }) => {
                        tracing::debug!(%name, %cid, "ANS update");
                        state_c.lock().insert(name, cid);
                    }
                    Err(e) => {
                        tracing::error!("Error decoding ANS record: {:?}", e);
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

    pub async fn set(&self, name: impl Into<String>, cid: Cid) -> anyhow::Result<Option<Cid>> {
        let name: String = name.into();
        let record = NameRecord {
            name: name.clone(),
            cid,
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

        let maybe_old = self.state.lock().insert(name, cid);
        Ok(maybe_old)
    }

    pub fn get(&self, name: &str) -> Option<Cid> {
        self.state.lock().get(name).cloned()
    }
}
