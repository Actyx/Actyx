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
use std::collections::BTreeMap;

use crate::{
    offset::{Offset, OffsetMap, OffsetOrMin},
    scalars::{NodeId, StreamId, StreamNr},
};
use quickcheck::{Arbitrary, Gen};

impl Arbitrary for NodeId {
    fn arbitrary(g: &mut Gen) -> Self {
        let x = u128::arbitrary(g);
        let mut bytes = [0u8; 32];
        bytes[0..16].copy_from_slice(&x.to_be_bytes());
        bytes[16..32].copy_from_slice(&x.to_le_bytes());
        NodeId(bytes)
    }
}

impl Arbitrary for StreamNr {
    fn arbitrary(g: &mut Gen) -> Self {
        u64::arbitrary(g).into()
    }
}

impl Arbitrary for StreamId {
    fn arbitrary(g: &mut Gen) -> Self {
        Self {
            node_id: NodeId::arbitrary(g),
            stream_nr: StreamNr::arbitrary(g),
        }
    }
}

impl Arbitrary for Offset {
    fn arbitrary(g: &mut Gen) -> Self {
        let offset: u32 = Arbitrary::arbitrary(g);
        Self::from(offset)
    }
}

impl Arbitrary for OffsetOrMin {
    fn arbitrary(g: &mut Gen) -> Self {
        if bool::arbitrary(g) {
            let offset: Offset = Arbitrary::arbitrary(g);
            Self::from(offset)
        } else {
            OffsetOrMin::MIN
        }
    }
}

impl Arbitrary for OffsetMap {
    fn arbitrary(g: &mut Gen) -> Self {
        let inner: BTreeMap<StreamId, Offset> = Arbitrary::arbitrary(g);
        Self::from(inner)
    }
}
