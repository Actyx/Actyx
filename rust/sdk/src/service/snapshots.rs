/*
 * Copyright 2021 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use serde::{Deserialize, Serialize};

use crate::{
    event::EventKey,
    legacy::{FishName, Semantics},
    offset::OffsetMap,
};

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
