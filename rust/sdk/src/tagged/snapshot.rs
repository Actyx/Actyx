use crate::types::Binary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Compression {
    None,
    Deflate,
}

#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotData {
    compression: Compression,
    data: Binary,
}

impl SnapshotData {
    pub fn new(compression: Compression, data: impl Into<Box<[u8]>>) -> Self {
        Self {
            compression,
            data: data.into().into(),
        }
    }
}
