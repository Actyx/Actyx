pub mod block;
pub mod cons_node;

pub use self::block::*;
pub use self::cons_node::*;

use actyx_sdk::{
    legacy::{FishName, Semantics},
    LamportTimestamp, Offset, Payload, TagSet, Timestamp,
};
use anyhow::Result;
use serde::{ser::Serializer, Deserialize, Deserializer, Serialize};
use std::{fmt::Debug, str, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IpfsEnvelope {
    pub semantics: Semantics,
    pub name: FishName,

    // Legacy compatibility: No value for field 'tags' means no tags.
    #[serde(default, skip_serializing_if = "TagSet::is_empty")]
    pub tags: TagSet,

    pub timestamp: Timestamp,
    #[serde(rename = "psn")]
    pub offset: Offset,
    pub payload: Payload,
    pub lamport: LamportTimestamp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::rc_buffer)]
pub struct EnvelopeList(Arc<Vec<IpfsEnvelope>>);

impl Serialize for EnvelopeList {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.as_ref().serialize(serializer)
    }
}

impl EnvelopeList {
    #[cfg(test)]
    pub fn single(envelope: IpfsEnvelope) -> EnvelopeList {
        EnvelopeList(Arc::new(vec![envelope]))
    }

    pub fn new(elements: Vec<IpfsEnvelope>) -> Option<EnvelopeList> {
        if !elements.is_empty() {
            Some(EnvelopeList(Arc::new(elements)))
        } else {
            None
        }
    }

    pub fn into_vec(self) -> Vec<IpfsEnvelope> {
        // try to get the content of the arc. If somebody else owns it, we have to clone.
        Arc::try_unwrap(self.0).unwrap_or_else(|err| err.as_ref().clone())
    }
}

impl<'de> Deserialize<'de> for EnvelopeList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let envelopes = Vec::<IpfsEnvelope>::deserialize(deserializer)?;
        EnvelopeList::new(envelopes)
            .ok_or_else(|| serde::de::Error::custom("envelope list must contain at least one element"))
    }
}

#[cfg(test)]
mod tests_v1 {
    use super::*;

    #[test]
    fn envelope_list_deser_invariants() {
        serde_json::from_str::<EnvelopeList>("[]").unwrap_err();
    }
}
