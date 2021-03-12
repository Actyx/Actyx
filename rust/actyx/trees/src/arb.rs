use std::{collections::BTreeSet, convert::TryFrom, ops::Range};

use actyxos_sdk::{
    tagged::{Tag, TagSet},
    LamportTimestamp, TimeStamp,
};
use quickcheck::{Arbitrary, Gen};

use crate::{
    axtrees::{AxKey, AxKeySeq, AxRange, AxSummary, AxSummarySeq, TagsSummary},
    TagIndex,
};

impl Arbitrary for AxKeySeq {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut tmp: Vec<(u64, u64)> = Arbitrary::arbitrary(g);
        tmp.push(Arbitrary::arbitrary(g));
        let tags: Vec<String> = Arbitrary::arbitrary(g);
        tmp.into_iter()
            .map(|(time, lamport)| {
                let time: TimeStamp = time.into();
                let lamport: LamportTimestamp = lamport.into();
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
                let time: AxRange<TimeStamp> = AxRange::new(time.start.into(), time.end.into());
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
                    tags: TagsSummary::from(&key_tags.into()),
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

impl Arbitrary for TagIndex {
    fn arbitrary(g: &mut Gen) -> Self {
        let xs: Vec<BTreeSet<IndexString>> = Arbitrary::arbitrary(g);
        let xs: Vec<TagSet> = xs
            .iter()
            .map(|e| e.iter().map(|x| Tag::try_from(x.0).unwrap()).collect())
            .collect();
        Self::from_elements(&xs)
    }
}
