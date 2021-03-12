use crate::BanyanStore;
use actyxos_sdk::{
    event::{SourceId, TimeStamp},
    Offset,
};
use parking_lot::Mutex;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tracing::debug;
use trees::{ConnectivityRequest, ConnectivityResponse, ConnectivityStatus};

#[derive(Clone)]
pub struct ConnectivityState {
    pub offset_gossip_about_us: Arc<Mutex<BTreeMap<SourceId, GossipAboutUs>>>,
    pub our_highest_offset: Arc<Mutex<Option<Offset>>>,
    pub events_to_read: Arc<AtomicU64>,
}

impl ConnectivityState {
    pub fn new() -> Self {
        Self {
            offset_gossip_about_us: Arc::new(Mutex::new(Default::default())),
            our_highest_offset: Arc::new(Mutex::new(None)),
            events_to_read: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GossipAboutUs {
    pub source_id: SourceId,
    pub offset: Offset,
    pub received_at: TimeStamp,
}

impl GossipAboutUs {
    pub fn create(source_id: SourceId, offset: Offset) -> Self {
        GossipAboutUs {
            source_id,
            offset,
            received_at: TimeStamp::now(), // the device's wall clock time at which offset was received (record created); used by connectivity info
        }
    }
}

/// Struct wrapping calculation of the connectivity status for
/// an individual node participating in a swarm.
/// After creation, consumers can use the `drive` function to
/// calculate a new state on demand. This obviously needs to
/// be driven from the outside.
///
/// Description of the algorithm used to determine the connectivity
/// state:
/// Simplified algorithm based only on offset values (Ingest).
/// A more comprehensive algorithm could also take into consideration
/// lamport clock values.
///
///  - fully connected: all Special are in Ingest and >90% of HbHist
///    are in Ingest (i.e., all Special and 90% of non-Special devices
///    have sent heartbeats with my latest offset in the last hb_hist_delay
///    microseconds)
///  - not connected: Ingest is empty (i.e., no devices have sent heartbeats
///    with my latest offset in the last hb_hist_delay microseconds)
///  - partially connected: neither fully nor not connected
///
///  We also persist certain information, e.g. "events in send buffer /
///  events to send" is the difference between our_highest_offset and
///  lowest of our offsets across all devices, "special not connected"
///  are the specials that are missing from ingest.
///
///  The calculation for how long we are in a given state will be done
///  by some external loop calling this method and sent at the API
///  level (see e.g. pond-service-api).
///
///  We also return the difference between highest and present, but this
///  is not very meaningful, as during the disconnect the tablet will
///  not know that highest does not reflect the reality at the other
///  tablet that has already progressed ;)
pub struct ConnectivityCalculator {
    gossip_about_us: Arc<Mutex<BTreeMap<SourceId, GossipAboutUs>>>,
    our_highest_offset: Arc<Mutex<Option<Offset>>>,
    events_to_read: Arc<AtomicU64>,
    history_delay: usize,
    offset_history: Vec<Option<Offset>>,
    previous_status: Option<ConnectivityStatus>,
    previous_status_timestamp: TimeStamp,
    in_current_status_for_ms: i64,
    request: ConnectivityRequest,
}

impl ConnectivityCalculator {
    pub fn new<C: Connectivity>(c: &C, req: ConnectivityRequest) -> ConnectivityCalculator {
        let history_delay = req.current_offset_history_delay as usize;
        Self {
            gossip_about_us: c.get_gossip(),
            our_highest_offset: c.get_our_highest_offset(),
            events_to_read: c.events_to_read(),
            history_delay,
            offset_history: vec![None; history_delay + 1],
            previous_status: None,
            previous_status_timestamp: TimeStamp::new(0),
            in_current_status_for_ms: 0,
            request: req,
        }
    }

    /// Calculate the next connectivity state based on the last,
    /// and returns it.
    pub fn drive(&mut self) -> ConnectivityResponse {
        let past_highest_offset = self.offset_history[self.history_delay];
        for i in (1..=self.history_delay).rev() {
            self.offset_history[i] = self.offset_history[i - 1];
        }
        self.offset_history[0] = *self.our_highest_offset.lock();
        let current_status: ConnectivityStatus = connectivity_status(
            &*self.gossip_about_us.lock(),
            &past_highest_offset,
            self.request.special.clone(),
            TimeStamp::now(),
            self.request.hb_hist_delay,
            self.events_to_read.load(Ordering::SeqCst),
        );
        let current_timestamp = TimeStamp::now(); // this is in micros
        if self.previous_status.is_some()
            && std::mem::discriminant(self.previous_status.as_ref().unwrap()) == std::mem::discriminant(&current_status)
        {
            let since_last_status = current_timestamp - self.previous_status_timestamp;
            self.in_current_status_for_ms += since_last_status / 1000;
        } else {
            self.in_current_status_for_ms = 0;
        }
        self.previous_status_timestamp = current_timestamp;
        self.previous_status = Some(current_status.clone());
        ConnectivityResponse {
            status: current_status,
            in_current_status_for_ms: self.in_current_status_for_ms as u64,
        }
    }
}

pub trait Connectivity: Clone + Send + Unpin + Sync + 'static {
    fn get_gossip(&self) -> Arc<Mutex<BTreeMap<SourceId, GossipAboutUs>>>;
    fn get_our_highest_offset(&self) -> Arc<Mutex<Option<Offset>>>;
    fn connectivity_status(&self, special: Vec<SourceId>, now: TimeStamp, hb_hist_delay: u64) -> ConnectivityStatus;
    fn events_to_read(&self) -> Arc<AtomicU64>;
}

/// this depends on the connectivity state being up to date
impl Connectivity for BanyanStore {
    fn get_gossip(&self) -> Arc<Mutex<BTreeMap<SourceId, GossipAboutUs>>> {
        self.0.connectivity.offset_gossip_about_us.clone()
    }

    fn get_our_highest_offset(&self) -> Arc<Mutex<Option<Offset>>> {
        self.0.connectivity.our_highest_offset.clone()
    }

    fn events_to_read(&self) -> Arc<AtomicU64> {
        self.0.connectivity.events_to_read.clone()
    }

    fn connectivity_status(
        &self,
        special: Vec<SourceId>,
        now: TimeStamp,
        hb_hist_delay: u64,
    ) -> trees::ConnectivityStatus {
        crate::connectivity::connectivity_status(
            &*self.get_gossip().lock(),
            &*self.get_our_highest_offset().lock(),
            special,
            now,
            hb_hist_delay,
            self.events_to_read().load(Ordering::SeqCst),
        )
    }
}

// the function below is static, to get around 'static lifetime issues at the point of usage (pond-service-api)
pub fn connectivity_status(
    offset_gossip: &BTreeMap<SourceId, GossipAboutUs>,
    our_highest_offset: &Option<Offset>,
    special: Vec<SourceId>,
    now: TimeStamp,
    hb_hist_delay: u64,
    events_to_read: u64,
) -> ConnectivityStatus {
    // Who has gossipped about us lately?
    let hb_hist: Vec<_> = offset_gossip
        .iter()
        .map(|e| e.1)
        .filter(|entry| entry.received_at + hb_hist_delay >= now)
        .collect();

    // What is the lowest offset that has been gossipped about us lately?
    let mut min_our_offset = Offset::MAX;
    for entry in &hb_hist {
        if entry.offset < min_our_offset {
            min_our_offset = entry.offset;
        }
    }

    // How many events are left to send to whoever is furthest behind?
    let events_to_send = our_highest_offset
        .filter(|our_offset| *our_offset > min_our_offset)
        .map(|our_offset| (our_offset - min_our_offset) as u64)
        .unwrap_or(0);

    // If we have sent any events, a list of those who have gossipped our latest event
    // Otherwise, an empty list
    let ingest: Vec<_> = if let Some(our_highest_offset) = our_highest_offset {
        hb_hist
            .iter()
            .filter(|entry| entry.offset >= *our_highest_offset)
            .collect()
    } else {
        hb_hist.iter().collect()
    };

    debug!(
        "min_our_offset {} events_to_send: {} ingest: {:?}, hb_hist: {:?}, offset_gossip: {:?}",
        min_our_offset, events_to_send, &ingest, &hb_hist, &offset_gossip
    );

    // Either we are read-only, no one has gossipped about us lately, or everyone who has gossipped
    // about us is out of date
    if ingest.is_empty() {
        return ConnectivityStatus::NotConnected {
            events_to_send,
            events_to_read,
        };
    }

    let special_set: HashSet<_> = special.into_iter().collect();
    let ingest_set: HashSet<_> = ingest.iter().map(|gossip| gossip.source_id).collect();

    let specials_disconnected: BTreeSet<_> = special_set.difference(&ingest_set).copied().collect();

    // All specials have gossipped our latest offset, and 90% of non-specials
    if (ingest.len() as f64 >= 0.9 * hb_hist.len() as f64) && specials_disconnected.is_empty() {
        return ConnectivityStatus::FullyConnected;
    }

    let swarm_connectivity_level = ((ingest.len() * 100) / hb_hist.len()) as u8;

    ConnectivityStatus::PartiallyConnected {
        swarm_connectivity_level,
        events_to_send,
        events_to_read,
        specials_disconnected: specials_disconnected.into_iter().collect::<Vec<_>>(),
    }
}
