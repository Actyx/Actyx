use crate::{actors::ComponentCommand, node_settings::Settings};
use acto::{ActoCell, ActoInput, ActoRuntime};
use actyx_sdk::{NodeId, Offset, OffsetMap, StreamId, Timestamp};
use im::OrdMap;
use ipfs_embed::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use swarm::{GossipMessage, RootMap, RootUpdate};
use util::variable::Writer;

pub enum SwarmObserver {
    NewSettings(Settings),
    Gossip(PeerId, RootMap),
    StreamUpdate(PeerId, RootUpdate),
}

impl From<ComponentCommand> for SwarmObserver {
    fn from(msg: ComponentCommand) -> Self {
        match msg {
            ComponentCommand::NewSettings(settings) => Self::NewSettings(settings),
        }
    }
}

impl From<(PeerId, GossipMessage)> for SwarmObserver {
    fn from((peer_id, msg): (PeerId, GossipMessage)) -> Self {
        match msg {
            GossipMessage::RootMap(x) => Self::Gossip(peer_id, x),
            GossipMessage::RootUpdate(x) => Self::StreamUpdate(peer_id, x),
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmState {
    peers_status: OrdMap<NodeId, Status>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// Acknowledge replication within two gossip cycles
    LowLatency,
    /// Acknowledge replication within five gossip cycles
    HighLatency,
    /// Acknowledge replication of at least half of all streams within
    /// five gossip cycles
    PartiallyWorking,
    /// Acknowledge replication of less than half of all streams within
    /// five gossip cycles
    NotWorking,
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    timestamp: Timestamp,
    offsets: OrdMap<StreamId, Offset>,
}

impl HistoryEntry {
    fn new(timestamp: Timestamp) -> Self {
        Self {
            timestamp,
            offsets: OrdMap::default(),
        }
    }

    fn ingest_map(&mut self, offsets: &OffsetMap) {
        for (stream_id, offset) in offsets.stream_iter() {
            let entry = self.offsets.entry(stream_id).or_default();
            if offset > *entry {
                *entry = offset;
            }
        }
    }

    fn ingest(&mut self, stream_id: StreamId, offset: Offset) {
        let entry = self.offsets.entry(stream_id).or_default();
        if offset > *entry {
            *entry = offset;
        }
    }
}

pub async fn swarm_observer(
    mut cell: ActoCell<SwarmObserver, impl ActoRuntime>,
    state: Writer<SwarmState>,
) -> anyhow::Result<()> {
    let mut history = Vec::<HistoryEntry>::new();
    let mut latest = HashMap::new();
    let mut peer_map = HashMap::new();
    let mut swarm_state = state.read().clone();
    let mut gossip_cycle_micros = 10_000_000;
    let mut lookback_low_latency = 2 * gossip_cycle_micros;
    let mut lookback_high_latency = 5 * gossip_cycle_micros;
    while let ActoInput::Message(msg) = cell.recv().await {
        let now = Timestamp::now();
        let fresh_cutoff = now - 1_000_000;
        match msg {
            SwarmObserver::NewSettings(settings) => {
                gossip_cycle_micros = settings.swarm.gossip_interval * 1_000_000;
                lookback_low_latency =
                    (settings.swarm.detection_cycles_low_latency * gossip_cycle_micros as f64) as u64;
                lookback_high_latency =
                    (settings.swarm.detection_cycles_high_latency * gossip_cycle_micros as f64) as u64;
                tracing::debug!(gossip = %gossip_cycle_micros, low = %lookback_low_latency, high = %lookback_high_latency, "new settings");
            }
            SwarmObserver::Gossip(peer_id, root_map) => {
                tracing::debug!(peer = %peer_id, "rootMap with {} streams", root_map.entries.len());
                if root_map.entries.len() == root_map.offsets.len() {
                    let offsets = to_offset_map(root_map);

                    // update overall swarm history
                    let he = latest_entry(&mut history, fresh_cutoff, now);
                    he.ingest_map(&offsets);

                    // keep track of who is who
                    for (stream_id, _) in offsets.stream_iter() {
                        let node_id = stream_id.node_id();
                        let peer_id = PeerId::from(crypto::PublicKey::from(node_id));
                        peer_map.insert(node_id, peer_id);
                    }

                    // store latest gossip from this node
                    latest.insert(peer_id, offsets);
                } else if !root_map.offsets.is_empty() {
                    tracing::warn!("inconsistent RootMap from {}", peer_id);
                }
            }
            SwarmObserver::StreamUpdate(peer_id, stream_update) => {
                tracing::debug!(peer = %peer_id, "rootUpdate");
                let stream_id = stream_update.stream;
                peer_map.insert(stream_id.node_id(), peer_id);
                if let Some(offset) = stream_update.offset {
                    let he = latest_entry(&mut history, fresh_cutoff, now);
                    he.ingest(stream_id, offset);
                }
            }
        }
        let low_latency = now - lookback_low_latency;
        let high_latency = now - lookback_high_latency;
        prune_history(&mut history, high_latency);
        if let (high, Some(low)) = (get_history(&history, high_latency), get_history(&history, low_latency)) {
            for (node_id, peer_id) in &peer_map {
                let empty = OffsetMap::empty();
                let offsets = latest.get(peer_id).unwrap_or(&empty);
                let (_present, absent) = check_streams(low, offsets);
                if absent == 0 {
                    set_state(&mut swarm_state, *node_id, Status::LowLatency);
                    continue;
                }
                // some streams were missing, try with the high-latency setting
                if let Some(high) = high {
                    let (present, absent) = check_streams(high, offsets);
                    if absent == 0 {
                        set_state(&mut swarm_state, *node_id, Status::HighLatency)
                    } else if absent <= present {
                        set_state(&mut swarm_state, *node_id, Status::PartiallyWorking)
                    } else {
                        set_state(&mut swarm_state, *node_id, Status::NotWorking)
                    }
                }
            }
        }
        tracing::trace!(state = ?swarm_state);
        *state.write() = swarm_state.clone();
    }
    Ok(())
}

fn set_state(swarm_state: &mut SwarmState, node_id: NodeId, status: Status) {
    if swarm_state.peers_status.get(&node_id) != Some(&status) {
        swarm_state
            .peers_status
            .entry(node_id)
            .and_modify(|s| *s = status)
            .or_insert(status);
    }
}

fn check_streams(low: &HistoryEntry, offsets: &OffsetMap) -> (u32, u32) {
    let mut present = 0;
    let mut absent = 0;
    for (stream_id, offset) in &low.offsets {
        if offsets.get(*stream_id) >= Some(*offset) {
            present += 1;
        } else {
            absent += 1;
        }
    }
    (present, absent)
}

fn latest_entry(history: &mut Vec<HistoryEntry>, fresh_cutoff: Timestamp, now: Timestamp) -> &mut HistoryEntry {
    if let Some(he) = history.last_mut() {
        if he.timestamp > fresh_cutoff {
            // nothing to do
        } else {
            let new = history.last().unwrap().clone();
            history.push(new);
        }
    } else {
        history.push(HistoryEntry::new(now));
    }
    history.last_mut().unwrap()
}

fn to_offset_map(root_map: RootMap) -> OffsetMap {
    root_map
        .entries
        .into_iter()
        .zip(root_map.offsets.into_iter())
        .map(|((stream, _), (offset, _))| (stream, offset))
        .collect()
}

fn prune_history(history: &mut Vec<HistoryEntry>, before: Timestamp) {
    if let Some(he) = history.get(1) {
        if he.timestamp <= before {
            let first_newer = history.partition_point(|he| he.timestamp <= before);
            history.drain(..first_newer - 1);
        }
    }
}

fn get_history(history: &[HistoryEntry], at: Timestamp) -> Option<&HistoryEntry> {
    let idx = history.partition_point(|he| he.timestamp <= at);
    if idx == 0 {
        None
    } else {
        Some(&history[idx - 1])
    }
}
