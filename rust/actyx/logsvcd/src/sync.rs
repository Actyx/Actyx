#[cfg(target_os = "android")]
use crate::android_logcat;
use crate::{
    error::{LogsvcdError, Result},
    formats::*,
    storage::{self, RetentionStategy, Storage, StorageWrapper},
    storage_query::StorageQuery,
    DynamicConfig, LogConfig,
};
use ::util::pinned_resource_sync::PinnedResourceSync;
use actyxos_lib::formats::logs::{LogEvent, LogRequest};
use actyxos_sdk::tagged::NodeId;
use chrono::Utc;
use crossbeam::channel::{self, Receiver, Sender};
use std::{ops::RangeInclusive, time::Duration};
use tracing::*;

// TODO: Maybe convert this crate to async
#[derive(Debug)]
pub enum LogSender {
    Sync(Sender<Vec<LogEvent>>),
    Async(tokio::sync::mpsc::Sender<Vec<LogEvent>>),
}
#[derive(Debug)]
pub struct GetLogRequest {
    query: Query,
    tx: LogSender,
}
impl GetLogRequest {
    pub fn new(query: Query) -> (Self, Receiver<Vec<LogEvent>>) {
        let (tx, rx) = channel::bounded(32);
        (
            Self {
                query,
                tx: LogSender::Sync(tx),
            },
            rx,
        )
    }
    pub fn new_async(query: Query) -> (Self, tokio::sync::mpsc::Receiver<Vec<LogEvent>>) {
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        (
            Self {
                query,
                tx: LogSender::Async(tx),
            },
            rx,
        )
    }
}
pub fn spawn_with_name<F, T>(name: String, f: F) -> std::thread::JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    std::thread::Builder::new()
        .name(name)
        .spawn(f)
        .expect("failed to spawn thread")
}

const LOG_BUFFER_COUNT: usize = 256;
const LOG_BUFFER_EMIT_EVERY_MS: usize = 50;

#[derive(Clone)]
pub struct LogServiceWrapper {
    pub tx: Sender<GetLogRequest>,
    pub publish_tx: Sender<LogRequest>,
}
impl LogServiceWrapper {
    pub fn new(opt: LogConfig, node_id: NodeId) -> Self {
        let (tx, rx) = channel::bounded(128);
        let (publish_tx, rx_pub) = channel::bounded(256);

        let ret = RetentionStategy::new(opt._static.retention_size, opt._static.retention_days);
        let config = storage::StorageConfig::new("UNINITIALIZED".into(), node_id, ret);
        let db_path = opt._static.db_path.clone();
        let storage = PinnedResourceSync::new(
            move || Storage::open(&db_path, config).expect("Unable to create SQLite db."),
            "LogServiceWrapper::Storage",
        );

        #[cfg(target_os = "android")]
        {
            // Writing to logcat
            let (request, rx_logs) = GetLogRequest::new(Query {
                mode: QueryMode::All,
                follow: true,
            });
            tx.send(request).unwrap();
            spawn_with_name("logcat_write_loop".into(), || {
                android_logcat::logs_to_logcat(rx_logs);
            });
            // Reading from logcat
            let storage = storage.clone();
            let lc_filter = opt._static.logcat_filter.clone();
            spawn_with_name("logcat_read_loop".into(), || {
                if let Err(e) = android_logcat::logcat_loop(storage, lc_filter) {
                    panic!("Error reading for logcat: {}", e);
                }
            });
        }

        spawn_with_name("log_service".into(), move || {
            let mut log_service = SyncLogService::spawn_new(storage.clone(), rx, rx_pub, opt.dynamic).unwrap();
            log_service.run();
        });

        Self { tx, publish_tx }
    }
}
pub struct SyncLogService {
    storage: PinnedResourceSync<Storage>,
    /// Channel from the database, signaling an inserted log.
    highest_seq: Receiver<i64>,
    /// Active log consumers.
    consumers: Vec<Sender<i64>>,
    /// New incoming log stream requests.
    requests: Receiver<GetLogRequest>,
    /// New incoming log insertion requests.
    post_logs: Receiver<LogRequest>,
    /// Channel signaling an update in the dynamic part of the config.
    dynamic_config: Receiver<DynamicConfig>,
}

impl SyncLogService {
    pub fn spawn_new(
        storage: StorageWrapper,
        requests: Receiver<GetLogRequest>,
        post_logs: Receiver<LogRequest>,
        dynamic_config: Receiver<DynamicConfig>,
    ) -> Result<Self> {
        let (tx, rx) = channel::bounded(128);
        storage.spawn_mut(|s| s.install_hook(tx))?;
        Ok(Self {
            storage,
            highest_seq: rx,
            consumers: vec![],
            requests,
            post_logs,
            dynamic_config,
        })
    }

    #[allow(clippy::cognitive_complexity)]
    fn stream_logs_int(
        storage: PinnedResourceSync<Storage>,
        highest_seq: Receiver<i64>,
        qm: Query,
        snd: LogSender,
    ) -> Result<()> {
        // -2 indicates unset
        let mut last_tip = None;
        while let Ok(live_seq) = if last_tip.is_none() {
            // Get highest seq directly from the DB, so we don't have to wait for
            // a new log to appear through `highest_seq`.
            Ok(storage.spawn_mut(|s| s.get_highest_seq(None))??)
        } else {
            highest_seq.recv()
        } {
            trace!("last_tip: {:?}, live_seq: {}", last_tip, live_seq);
            let qmc = qm.clone();
            let bounds = storage.spawn_mut(move |s| get_bounds(qmc, &s, last_tip, live_seq))??;
            trace!("bounds {} {}", bounds.start(), bounds.end());

            let mut from = *bounds.start();
            // Query and send in batches of 10.
            let logs = StorageQuery::new(bounds, &storage, 100);

            for chunk in logs {
                let to = from + chunk.len() as i64;
                trace!("sub-query bounds {} {}", from, to);
                from = to;

                // This is running on a dedicated thread, so we can wait ..
                let is_err = match &snd {
                    LogSender::Sync(s) => s.send(chunk).is_err(),
                    LogSender::Async(s) => s.blocking_send(chunk).is_err(),
                };
                if is_err {
                    return Err(LogsvcdError::GenericError(
                        "Sender dropped (this usually means that the client cancelled the request).".into(),
                    ));
                }
            }

            // If the request was bounded, return.
            // This will drop `snd`, which indicates EOF to the receiving end.
            if !qm.follow {
                return Ok(());
            }
            last_tip = Some(live_seq);
        }
        Ok(())
    }
    /// Streams logs to `snd` according to `qm`.
    fn stream_logs(&mut self, qm: Query, snd: LogSender) {
        let (tx, rx) = channel::bounded(128);
        self.consumers.push(tx);
        let storage = self.storage.clone();
        spawn_with_name("handle_get_log_request".into(), move || {
            if let Err(e) = Self::stream_logs_int(storage, rx, qm, snd) {
                error!("Error handling query: {}.", e)
            }
        });
    }
    /// Notifies all consumers about a new inserted log with sequence no. `seq`.
    fn notify_new_log(&mut self, seq: i64) {
        let consumers = std::mem::replace(&mut self.consumers, vec![]);
        self.consumers = consumers.into_iter().filter(|s| s.send(seq).is_ok()).collect();
    }
    /// Handles a log insertion request. Consumes all log entries from `logs`,
    /// leaving an empty vec in place.
    fn handle_log_insert(&mut self, logs: &mut Vec<LogRequest>) -> Result<()> {
        trace!("Trying to insert {} logs, draining buffer.", logs.len());
        let req = std::mem::replace(logs, vec![]);
        self.storage.spawn_mut(|s| s.add_logs(req))??;
        Ok(())
    }

    /// Returns true on a fatal error
    fn err_fatal<T, F, E>(m: std::result::Result<T, E>, f: F) -> bool
    where
        F: FnOnce(T),
        E: Into<LogsvcdError>,
    {
        match m {
            Ok(x) => {
                f(x);
                false
            }
            Err(_) => {
                info!("LOGSVCD feeder channel closed, assuming shutdown in progress.");
                true
            }
        }
    }
    fn handle_new_config(&self, cfg: DynamicConfig) {
        debug!("Received new dynamic config {:?}", cfg);
        let node_name = cfg.node_name;
        let node_id = cfg.node_id;
        if let Err(e) = self.storage.spawn_mut(move |s| {
            s.change_config(|cfg| {
                cfg.node_name = node_name;
                cfg.node_id = node_id;
            })
        }) {
            error!("Error setting config on storage: {}", e);
        }
    }
    #[allow(clippy::blocks_in_if_conditions)]
    pub fn run(&mut self) {
        let prune_ticker = channel::tick(Duration::from_secs(30 * 60));
        // Enqueueuing new logs should be very fast, as the emission channels
        // are to be drained quickly. In case of high frequent logging, the
        // requests are going to be batched here and periodically written to the
        // database.
        let batch_ticker = channel::tick(Duration::from_millis(LOG_BUFFER_EMIT_EVERY_MS as u64));
        let mut batched_logs = vec![];

        loop {
            channel::select! {
                recv(batch_ticker) -> _ => {
                    if !batched_logs.is_empty() {
                        if let Err(e) = self.handle_log_insert(&mut batched_logs) {
                            error!("Error handling log insertion: {}", e);
                        }
                    }
                },
                recv(self.requests) -> msg =>
                    if Self::err_fatal(msg, |req| self.stream_logs(req.query, req.tx)) { break; },
                recv(self.highest_seq) -> msg =>
                    if Self::err_fatal(msg, |seq| self.notify_new_log(seq)) { break; },
                recv(self.post_logs) -> msg =>
                    if Self::err_fatal(msg, |req| {
                        batched_logs.push(req);
                        if batched_logs.len() >=  LOG_BUFFER_COUNT {
                            if let Err(e) = self.handle_log_insert(&mut batched_logs) {
                                error!("Error handling log insertion: {}", e);
                            }
                        }})
                        {
                            break;
                        },
                recv(prune_ticker) -> _ =>
                    if let Err(e) = prune(&self.storage) {
                        error!("Error pruning DB: {}", e);
                },
                recv(self.dynamic_config) -> msg =>
                    if Self::err_fatal(msg, |req| self.handle_new_config(req, )) { break; },
            }
        }
    }
}

fn prune(storage: &StorageWrapper) -> Result<()> {
    storage.spawn_mut(|s| s.prune())?
}

type Bounds = RangeInclusive<i64>;
use chrono::DateTime;
fn get_bounds_by_time<F>(
    since: DateTime<Utc>,
    to: Option<DateTime<Utc>>,
    last: Option<i64>,
    live_seq: i64,
    get_highest_seq: F,
) -> Result<Bounds>
where
    F: Fn(Option<DateTime<Utc>>) -> Result<i64>,
{
    let from = last
        // This will never be unwrapped, as there's the `or_else` below.
        .ok_or(())
        .or_else(|_| get_highest_seq(Some(since)))?;
    let to = match to {
        Some(max_time) => {
            let upper_bound = max_time.min(Utc::now());
            get_highest_seq(Some(upper_bound))?
        }
        _ => live_seq,
    };
    let to = to.max(0);
    Ok(from..=to)
}
pub(crate) fn get_bounds(
    query: Query,
    store: &Storage,
    last: Option<i64>,
    live_seq: i64,
) -> Result<RangeInclusive<i64>> {
    match query.mode {
        QueryMode::ByTime { since, to } => get_bounds_by_time(since, to, last, live_seq, |t| store.get_highest_seq(t)),
        QueryMode::MostRecent { count } => {
            // inclusive upper and lower bound
            let from = last
                .map(|x| x + 1)
                .unwrap_or_else(|| live_seq - count as i64 + 1)
                .max(0);
            let to = live_seq.max(0);
            trace!("bounds: from: {}, to: {}", from, to);
            if from > to {
                return Err(format!("Upper bound {} > lower bound {}", from, to).into());
            }

            Ok(from..=to)
        }
        QueryMode::All => {
            let from = last.map(|x| x + 1).unwrap_or(0);
            let to = live_seq.max(0);
            debug!("bounds: from: {}, to: {}", from, to);
            if from > to {
                return Err(format!("Upper bound {} > lower bound {}", from, to).into());
            }

            Ok(from..=to)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::storage::{test::insert_dummy_logs, Storage};
    use chrono::Utc;

    #[test]
    fn should_return_proper_bounds() {
        let mut s = Storage::in_memory();

        let cnt = 4242;
        insert_dummy_logs(&mut s, cnt);
        assert_eq!(cnt, s.get_highest_seq(None).unwrap());

        let qm = Query {
            mode: QueryMode::MostRecent { count: 100 },
            follow: true,
        };
        assert_eq!(get_bounds(qm, &s, None, cnt).unwrap(), cnt - 99..=cnt);

        let qm = Query {
            mode: QueryMode::MostRecent { count: 100 },
            follow: true,
        };
        // last seen overwrites most recent
        assert_eq!(get_bounds(qm, &s, Some(90), cnt).unwrap(), 91..=cnt);

        let qm = Query {
            mode: QueryMode::All,
            follow: true,
        };
        assert_eq!(get_bounds(qm, &s, None, cnt).unwrap(), 0..=cnt);

        let qm = Query {
            mode: QueryMode::All,
            follow: true,
        };
        assert_eq!(get_bounds(qm, &s, Some(4000), cnt).unwrap(), 4001..=cnt);

        let qm = Query {
            mode: QueryMode::ByTime {
                since: Utc::now(),
                to: None,
            },
            follow: false,
        };
        assert_eq!(get_bounds(qm, &s, None, cnt).unwrap(), cnt..=cnt);
    }
}
