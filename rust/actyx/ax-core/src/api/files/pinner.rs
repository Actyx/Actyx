use crate::{
    api::EventService,
    swarm::{Block, Ipfs},
};
use anyhow::Context;
use ax_aql::{Query, StaticQuery};
use ax_types::{
    app_id,
    service::{
        EventMeta, EventResponse, Order, PublishEvent, PublishRequest, QueryRequest, QueryResponse, SubscribeRequest,
        SubscribeResponse,
    },
    tags, AppId, Metadata, Payload, Timestamp,
};
use chrono::Utc;
use futures::{pin_mut, stream, Future, StreamExt};
use libipld::{cbor::DagCborCodec, multihash::Code, Cid, DagCbor};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::TryFrom,
    sync::Arc,
    time::Duration,
};
use tokio::{sync::mpsc, task::JoinHandle, time::MissedTickBehavior};
use tokio_stream::wrappers::{IntervalStream, ReceiverStream};

type UpdatePrefetch = (AppId, Query<'static>);

#[derive(Clone)]
pub struct FilePinner {
    tx: mpsc::Sender<UpdatePrefetch>,
    handle: Arc<JoinHandle<()>>,
}
impl Drop for FilePinner {
    fn drop(&mut self) {
        if Arc::strong_count(&self.handle) == 1 {
            self.handle.abort()
        }
    }
}

#[derive(Clone, DagCbor)]
struct RootLinkNode(Vec<Cid>);

#[derive(Serialize, Deserialize)]
enum FilePrefetchEvent {
    PinAdded {
        app_id: AppId,
        #[serde(deserialize_with = "deser_query")]
        query: Query<'static>,
        duration: Duration,
    },
    FutureCompact,
}
fn deser_query<'de, D: Deserializer<'de>>(d: D) -> Result<Query<'static>, D::Error> {
    Ok(StaticQuery::deserialize(d)?.0)
}

#[derive(PartialEq)]
struct StandingQuery {
    created: Timestamp,
    duration: Duration,
    query: String,
}

impl FilePinner {
    pub(crate) fn new(event_svc: EventService, ipfs: Ipfs) -> Self {
        let (tx, rx) = mpsc::channel::<UpdatePrefetch>(64);

        // TODO
        let retention = Duration::from_secs(60 * 60 * 24 * 7);
        let handle = tokio::spawn(async move {
            let event_svc = event_svc;
            let subscription = event_svc
                .subscribe(
                    app_id!("com.actyx"),
                    SubscribeRequest {
                        lower_bound: None,
                        query: "FROM isLocal & appId(com.actyx) & 'files:pinned'"
                            .parse()
                            .expect("valid syntax"),
                    },
                )
                .await
                .expect("Error opening subscription to store")
                .fuse();

            let mut query_interval = tokio::time::interval(Duration::from_secs(10 * 60));
            query_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
            let mut standing_queries: BTreeMap<AppId, StandingQuery> = Default::default();
            enum O {
                Update(UpdatePrefetch),
                Subscription(SubscribeResponse),
                Tick,
            }
            let mut s = stream::select_all([
                ReceiverStream::new(rx).map(O::Update).boxed(),
                subscription.map(O::Subscription).boxed(),
                IntervalStream::new(query_interval).map(|_| O::Tick).boxed(),
            ]);
            while let Some(output) = s.next().await {
                match output {
                    O::Update((app_id, query)) => {
                        tracing::debug!(%app_id, %query, "Received Update");
                        if let Err(error) = publish_update(&event_svc, app_id.clone(), query, retention).await {
                            tracing::error!(%app_id, %error, "Error updating pin");
                        }
                    }
                    O::Subscription(r) => {
                        match update_query(&mut standing_queries, r) {
                            Err(error) => tracing::error!(%error, "Error evaluating query"),
                            Ok(true) => {
                                // Also check the queries if something changed
                                check_queries(&event_svc, &ipfs, &mut standing_queries).await
                            }
                            _ => {}
                        }
                    }
                    O::Tick => check_queries(&event_svc, &ipfs, &mut standing_queries).await,
                }
            }
        });
        let slf = Self {
            tx,
            handle: Arc::new(handle),
        };
        tokio::spawn(slf.pin_internal_loop());
        slf
    }

    fn pin_internal_loop(&self) -> impl Future<Output = ()> + 'static {
        let tx = self.tx.clone();
        async move {
            // Pin all locally added files within the last 12 hours.
            loop {
                // TODO: We might want to make this period configurable.
                let now = Utc::now() - chrono::Duration::hours(12);
                if let Err(error) = tx
                    .send((
                        app_id!("com.actyx"),
                        Query::parse(&format!(
                            r#"
FEATURES(zøg aggregate timeRange)
FROM isLocal &
     appId(com.actyx) &
     'files:created' &
     from({})
SELECT _.cid"#,
                            now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                        ))
                        .expect("valid query")
                        .forget_pragmas(),
                    ))
                    .await
                {
                    tracing::error!(%error,"Error updating internal retention query");
                }
                tokio::time::sleep(Duration::from_secs(60 * 30)).await;
            }
        }
    }

    pub fn update(&self, app_id: AppId, query: Query<'static>) -> impl Future<Output = anyhow::Result<()>> + 'static {
        let tx = self.tx.clone();
        async move {
            tx.send((app_id, query)).await?;
            Ok(())
        }
    }
}

async fn check_queries(event_svc: &EventService, ipfs: &Ipfs, standing_queries: &mut BTreeMap<AppId, StandingQuery>) {
    tracing::trace!("Evaluating standing queries");
    let now = Timestamp::now();
    let mut app_ids_to_clear = vec![];
    standing_queries.retain(|k, q| {
        if q.created + q.duration > now {
            true
        } else {
            app_ids_to_clear.push(k.clone());
            false
        }
    });
    for app_id in app_ids_to_clear {
        if let Err(error) = ipfs.alias(AppPinAlias::from(&app_id), None) {
            tracing::error!(%app_id, %error, "Error clearing pin");
        }
    }
    for (app_id, query) in standing_queries {
        if let Err(error) = evaluate(event_svc, ipfs, app_id, query.query.clone()).await {
            tracing::error!(%error, %app_id, "Error updating standing query");
        }
    }
}
struct AppPinAlias(Vec<u8>);
impl From<&AppId> for AppPinAlias {
    fn from(app_id: &AppId) -> Self {
        Self(format!("app_pin:{}", app_id).as_bytes().to_vec())
    }
}
impl AsRef<[u8]> for AppPinAlias {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
async fn evaluate(event_svc: &EventService, ipfs: &Ipfs, app_id: &AppId, query: String) -> anyhow::Result<()> {
    let s = event_svc
        .query(
            app_id!("com.actyx"),
            QueryRequest {
                lower_bound: None,
                upper_bound: None,
                query,
                order: Order::Desc,
            },
        )
        .await?
        .filter_map(|r| async move {
            if let QueryResponse::Event(EventResponse { payload, .. }) = r {
                Some(payload)
            } else {
                None
            }
        });
    pin_mut!(s);
    let mut cids = BTreeSet::default();
    while let Some(payload) = s.next().await {
        let cid = payload
            .extract::<String>()
            .context("Extracting String from query")
            .and_then(|s| Cid::try_from(&*s).map_err(Into::into))
            .with_context(|| format!("Query for {} failed. Expected: Cid", app_id))?;

        cids.insert(cid);
    }

    if !cids.is_empty() {
        let root = RootLinkNode(cids.into_iter().collect());
        let block = Block::encode(DagCborCodec, Code::Blake3_256, &root)?;
        let cid = *block.cid();
        ipfs.insert(block)?;
        ipfs.alias(AppPinAlias::from(app_id), Some(&cid))?;
        tracing::debug!(root = %cid, %app_id, "Updated pinned files");
    }
    Ok(())
}

async fn publish_update(
    event_svc: &EventService,
    app_id: AppId,
    query: Query<'static>,
    duration: Duration,
) -> anyhow::Result<()> {
    event_svc
        .publish(
            app_id!("com.actyx"),
            PublishRequest {
                data: vec![PublishEvent {
                    tags: tags!("files", "files:pinned"),
                    payload: Payload::compact(&FilePrefetchEvent::PinAdded {
                        app_id,
                        duration,
                        query,
                    })?,
                }],
            },
        )
        .await?;
    Ok(())
}

fn update_query(
    standing_queries: &mut BTreeMap<AppId, StandingQuery>,
    event: SubscribeResponse,
) -> anyhow::Result<bool> {
    if let SubscribeResponse::Event(EventResponse {
        meta: EventMeta::Event {
            meta: Metadata { timestamp: created, .. },
            ..
        },
        payload,
    }) = event
    {
        if let FilePrefetchEvent::PinAdded {
            app_id,
            duration,
            query,
        } = payload.extract()?
        {
            let now = Timestamp::now();
            let q = StandingQuery {
                created,
                duration,
                query: query.to_string(),
            };

            if created + duration > now && standing_queries.get(&app_id) != Some(&q) {
                tracing::trace!(%app_id, ?duration, query=%q.query, "Updated standing query");
                standing_queries.insert(app_id, q);
                return Ok(true);
            }
        }
    }
    Ok(false)
}
