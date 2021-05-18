use crate::SwarmOffsets;
use actyxos_sdk::{service::OffsetsResponse, OffsetOrMin};
use std::{convert::TryFrom, num::NonZeroU64};

impl From<&SwarmOffsets> for OffsetsResponse {
    fn from(o: &SwarmOffsets) -> Self {
        let to_replicate = o
            .replication_target
            .stream_iter()
            .filter_map(|(stream, target)| {
                let actual = o.present.offset(stream);
                let diff = OffsetOrMin::from(target) - actual;
                u64::try_from(diff).ok().and_then(NonZeroU64::new).map(|o| (stream, o))
            })
            .collect();

        Self {
            present: o.present.clone(),
            to_replicate,
        }
    }
}
