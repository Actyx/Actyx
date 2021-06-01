use std::{convert::TryFrom, ops::Range};

use actyxos_sdk::{LamportTimestamp, Tag, TagSet, Timestamp};
use quickcheck::{Arbitrary, Gen};

use crate::axtrees::{AxKey, AxKeySeq, AxRange, AxSummary, AxSummarySeq, TagsSummary};

impl Arbitrary for AxKey {
    fn arbitrary(g: &mut Gen) -> Self {
        let tags = TagSet::arbitrary(g);
        Self {
            tags,
            lamport: u64::arbitrary(g).into(),
            time: u64::arbitrary(g).into(),
        }
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let Self { lamport, tags, time } = self.clone();
        // Let's assume only tags matter..
        Box::new(tags.shrink().map(move |tags| Self { tags, lamport, time }))
    }
}

impl Arbitrary for AxKeySeq {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut tmp: Vec<(u64, u64)> = Arbitrary::arbitrary(g);
        tmp.push(Arbitrary::arbitrary(g));
        let tags: Vec<String> = Arbitrary::arbitrary(g);
        tmp.into_iter()
            .map(|(time, lamport)| {
                let time: Timestamp = time.into();
                let lamport: LamportTimestamp = lamport.into();
                let mut key_tags: Vec<Tag> = vec![];
                for x in 0..*g.choose(&[0, 1, 2, 3]).unwrap() {
                    if x >= tags.len() {
                        break;
                    }
                    if tags[x].is_empty() {
                        continue;
                    }
                    key_tags.push(Tag::try_from(tags[x].as_str()).unwrap());
                }
                AxKey {
                    time,
                    lamport,
                    tags: key_tags.into(),
                }
            })
            .collect()
    }
}

impl Arbitrary for AxSummarySeq {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut tmp: Vec<(Range<u64>, Range<u64>)> = Arbitrary::arbitrary(g);
        tmp.push(Arbitrary::arbitrary(g));
        let tags: Vec<String> = Arbitrary::arbitrary(g);
        tmp.into_iter()
            .map(|(time, lamport)| {
                let time: AxRange<Timestamp> = AxRange::new(time.start.into(), time.end.into());
                let lamport: AxRange<LamportTimestamp> = AxRange::new(lamport.start.into(), lamport.end.into());
                let mut key_tags = vec![];
                for x in 0..*g.choose(&[0, 1, 2, 3]).unwrap() {
                    if x >= tags.len() {
                        break;
                    }
                    if tags[x].is_empty() {
                        continue;
                    }
                    key_tags.push(Tag::try_from(tags[x].as_str()).unwrap());
                }
                AxSummary {
                    time,
                    lamport,
                    tags: TagsSummary::from_slice(key_tags.as_ref()),
                }
            })
            .collect()
    }
}

const STRINGS: &[&str] = &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq)]
struct IndexString(&'static str);

impl Arbitrary for IndexString {
    fn arbitrary(g: &mut Gen) -> Self {
        IndexString(g.choose(STRINGS).unwrap())
    }
}
