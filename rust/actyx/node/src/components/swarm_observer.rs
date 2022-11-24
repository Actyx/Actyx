use crate::{actors::ComponentCommand, node_settings::Settings};
use acto::{ActoCell, ActoInput, ActoRuntime};
use actyx_sdk::{LamportTimestamp, NodeId, Offset, StreamId, Timestamp};
use cbor_data::codec::{ReadCbor, WriteCbor};
use im::{OrdMap, OrdSet};
use ipfs_embed::PeerId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use swarm::{RootMap, RootUpdate};
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

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmState {
    live_peers: OrdSet<NodeId>,
    peers_status: OrdMap<NodeId, Status>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default)]
struct NodeInfo {
    time: Timestamp,
    lamport: LamportTimestamp,
    offsets: OrdMap<StreamId, (LamportTimestamp, Offset)>,
}

pub async fn swarm_observer(
    mut cell: ActoCell<SwarmObserver, impl ActoRuntime>,
    state: Writer<SwarmState>,
) -> anyhow::Result<()> {
    let mut node_infos = BTreeMap::<PeerId, NodeInfo>::new();
    let mut streams = BTreeSet::<StreamId>::new();
    let mut nodes = BTreeMap::<NodeId, LamportTimestamp>::new();
    let mut swarm_state = SwarmState::default();
    while let ActoInput::Message(msg) = cell.recv().await {
        match msg {
            SwarmObserver::NewSettings(_) => todo!(),
            SwarmObserver::Gossip(peer_id, root_map) => {
                let entry = node_infos.entry(peer_id).or_default();
                entry.time = root_map.time;
                entry.lamport = root_map.lamport;
                if root_map.entries.len() == root_map.offsets.len() {
                    entry.offsets = root_map
                        .entries
                        .into_iter()
                        .zip(root_map.offsets.into_iter())
                        .inspect(|((id, _), (_, ts))| {
                            streams.insert(*id);
                            let t = nodes.entry(id.node_id()).or_default();
                            *t = (*t).max(*ts);
                        })
                        .map(|((stream, _), (offset, lamport))| (stream, (lamport, offset)))
                        .collect();
                }
            }
            SwarmObserver::StreamUpdate(peer_id, stream_update) => {
                let entry = node_infos.entry(peer_id).or_default();
                entry.time = stream_update.time;
                entry.lamport = stream_update.lamport;
                let offsets = entry.offsets.entry(stream_update.stream).or_default();
                offsets.0 = offsets.0.max(stream_update.lamport);
                if let Some(offset) = stream_update.offset {
                    offsets.1 = offsets.1.max(offset);
                }
                streams.insert(stream_update.stream);
                let lt = nodes.entry(stream_update.stream.node_id()).or_default();
                *lt = (*lt).max(stream_update.lamport);
            }
        }
        *state.write() = swarm_state.clone();
    }
    Ok(())
}
