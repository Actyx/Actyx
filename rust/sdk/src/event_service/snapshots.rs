use crate::{
    event::{FishName, Semantics, SourceId},
    LamportTimestamp, Offset, OffsetMap,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoreSnapshotRequest {
    pub semantics: Semantics,
    pub name: FishName,
    pub key: EventKeyV1,
    pub psn_map: OffsetMap,
    pub horizon: Option<EventKeyV1>,
    pub cycle: u64,
    pub version: u64,
    pub tag: String,
    pub blob: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RetrieveSnapshotRequest {
    pub semantics: Semantics,
    pub name: FishName,
    pub version: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InvalidateSnapshotsRequest {
    pub semantics: Semantics,
    pub name: FishName,
    pub key: EventKeyV1,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RetrieveSnapshotResponse {
    pub state: String,
    pub psn_map: OffsetMap,
    pub event_key: EventKeyV1,
    pub horizon: Option<EventKeyV1>,
    pub cycle: u64,
}

/// Event key used in the snapshot endpoints of the pond service
///
/// to be replaced with actyxos_sdk::EventKey for v2
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Copy)]
#[serde(rename_all = "camelCase")]
pub struct EventKeyV1 {
    pub lamport: LamportTimestamp,
    pub source_id: SourceId,
    #[serde(rename = "psn")]
    pub offset: Offset,
}
