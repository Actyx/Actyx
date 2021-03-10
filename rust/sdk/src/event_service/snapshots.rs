use crate::{
    event::{FishName, Semantics},
    tagged::EventKey,
    OffsetMap,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoreSnapshotRequest {
    pub entity_type: Semantics,
    pub name: FishName,
    pub key: EventKey,
    pub offset_map: OffsetMap,
    pub horizon: Option<EventKey>,
    pub cycle: u64,
    pub version: u64,
    pub tag: String,
    pub blob: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RetrieveSnapshotRequest {
    pub entity_type: Semantics,
    pub name: FishName,
    pub version: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InvalidateSnapshotsRequest {
    //TODO: Create `EntityType` type
    pub entity_type: Semantics,
    //TODO: Create `Name` type
    pub name: FishName,
    pub key: EventKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RetrieveSnapshotResponse {
    pub state: String,
    pub offset_map: OffsetMap,
    pub event_key: EventKey,
    pub horizon: Option<EventKey>,
    pub cycle: u64,
}
