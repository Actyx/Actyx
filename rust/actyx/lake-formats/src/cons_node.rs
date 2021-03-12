use crate::{Block, EnvelopeList, Offset, TagIndex};
use actyxos_sdk::event::{FishName, Semantics};
use libipld::{DagCbor, Link};
use std::collections::BTreeMap;
use std::convert::TryInto;

#[derive(Clone, DagCbor, Debug, PartialEq, Eq)]
pub struct ConsNode {
    /// The minimum offset of the block. min === head(blocks).offset
    pub min: Offset,

    /// The maximum offset of the block. max === last(blocks).offset
    pub max: Offset,

    /// Link to a block of events
    ///
    /// The block must be a contiguous array of events, with ascending
    /// offsets from min to max. So total size of the array must be max-min+1
    pub block: Link<Block>,

    /// Optional link to the previous ConsNode.
    ///
    /// If prev is defined, min === prev.max + 1. So by following prev you
    /// go into the past.
    #[ipld(default = None)]
    pub prev: Option<Link<ConsNode>>,

    /// Optional index data structure containing mappings from semantics to
    /// distinct, sorted arrays of fish names
    pub index: Option<BlockIndex>,

    /// Optional field to indicate that a block is compacted according to a certain compaction algorithm
    #[ipld(rename = "compactedWith")]
    #[ipld(default = None)]
    pub compacted_with: Option<String>,

    /// Optional index data structure containing tags for every index.
    #[ipld(rename = "tagIndex")]
    #[ipld(default = None)]
    pub tag_index: Option<TagIndex>,
}

impl ConsNode {
    pub fn num_covered_offsets(&self) -> usize {
        ((self.max - self.min) + 1)
            .try_into()
            .unwrap_or_else(|_| panic!("Extremely weird max/min in block {:?}", self.block))
    }
}

#[derive(Clone, DagCbor, Debug, Default, PartialEq, Eq)]
#[ipld(repr = "value")]
pub struct BlockIndex(pub BTreeMap<Semantics, Vec<FishName>>);

impl BlockIndex {
    pub fn new(envelope_list: &EnvelopeList) -> Self {
        let mut index = Self::empty();
        index.add(envelope_list);
        index
    }

    fn empty() -> Self {
        BlockIndex(BTreeMap::new())
    }

    pub fn add(&mut self, envelope_list: &EnvelopeList) {
        let map = &mut self.0;

        envelope_list.0.iter().for_each(|envelope| {
            let fish_names = map.entry(envelope.semantics.clone()).or_insert_with(Vec::new);
            if !fish_names.contains(&envelope.name) {
                fish_names.push(envelope.name.clone());
            };
        });

        map.values_mut().for_each(|names| names.sort());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlockIndex, IpfsEnvelope};
    use actyxos_sdk::{
        event::{LamportTimestamp, Payload, TimeStamp},
        fish_name, semantics, tags,
    };
    use libipld::cbor::DagCborCodec;
    use libipld::codec::{Codec, Decode, Encode};
    use libipld::multihash::{Code, MultihashDigest};
    use libipld::Cid;
    use maplit::*;
    use rand::Rng;

    fn link<T>(data: &[u8]) -> Link<T> {
        let hash = Code::Sha2_256.digest(data);
        Link::new(Cid::new_v1(DagCborCodec.into(), hash))
    }

    fn serde<T: Encode<DagCborCodec> + Decode<DagCborCodec>>(data: &T) -> T {
        let bytes = DagCborCodec.encode(data).unwrap();
        DagCborCodec.decode(&bytes).expect("Should be able to deserialize")
    }

    #[test]
    fn serialize_deserialize_index_cbor() {
        let index = BlockIndex(btreemap! {
            semantics!("foo") => vec![fish_name!("A")],
            semantics!("bar") => vec![fish_name!("A"), fish_name!("A")],
            semantics!("baz") => vec![],
        });

        let index2 = serde(&index);
        assert_eq!(index.0, index2.0);
    }

    #[test]
    fn serialize_deserialize_cons_node_cbor() {
        let index = BlockIndex(btreemap! {
            semantics!("foo") => vec![fish_name!("A")],
            semantics!("bar") => vec![fish_name!("A"), fish_name!("A")],
            semantics!("baz") => vec![],
        });
        let block_link = link(&b"zabcd"[..]);
        let cons_link = link(&b"zefgh"[..]);

        let cons_node1 = ConsNode {
            min: Offset::mk_test(123),
            max: Offset::mk_test(8876),
            block: block_link,
            prev: None,
            index: None,
            compacted_with: None,
            tag_index: None,
        };

        let cons_node1_out = serde(&cons_node1);
        assert_eq!(cons_node1_out.min, Offset::mk_test(123));
        assert_eq!(cons_node1_out.max, Offset::mk_test(8876));
        assert_eq!(cons_node1_out.block.cid(), block_link.cid());
        assert!(cons_node1_out.prev.is_none());
        assert!(cons_node1_out.index.is_none());

        let tag_index = TagIndex::from_elements(&[tags! { "tag0", "tag1" }, tags! { "tag2" }]);

        let cons_node2 = ConsNode {
            min: Offset::mk_test(123),
            max: Offset::mk_test(8876),
            block: block_link,
            prev: Some(cons_link),
            index: Some(index.clone()),
            tag_index: Some(tag_index.clone()),
            compacted_with: Some("foobar".to_string()),
        };

        let cons_node2_out = serde(&cons_node2);
        assert_eq!(cons_node2_out.min, Offset::mk_test(123));
        assert_eq!(cons_node2_out.max, Offset::mk_test(8876));
        assert_eq!(cons_node2_out.block.cid(), block_link.cid());
        assert_eq!(cons_node2_out.prev.unwrap().cid(), cons_link.cid());
        assert_eq!(cons_node2_out.index.unwrap().0, index.0);
        assert_eq!(cons_node2_out.tag_index.unwrap(), tag_index);
        assert_eq!(cons_node2_out.compacted_with.unwrap(), "foobar");
    }

    fn envelope(semantics: &Semantics, name: String) -> IpfsEnvelope {
        let name = FishName::new(name).unwrap();
        IpfsEnvelope {
            tags: tags! { semantics, &name },
            semantics: semantics.clone(),
            name,
            timestamp: TimeStamp::new(0),
            lamport: LamportTimestamp::new(0),
            offset: Offset::ZERO,
            payload: Payload::empty(),
        }
    }

    fn create_envelopes(semantics: Vec<&Semantics>, fish_count: usize) -> EnvelopeList {
        let mut envelopes: Vec<IpfsEnvelope> = Vec::new();
        let evs = &mut envelopes;
        let mut rng = rand::thread_rng();

        semantics.iter().for_each(move |semantics| {
            let start = rng.gen_range(0..fish_count);
            for i in 0..fish_count {
                let idx = (start + i) % fish_count;
                evs.push(envelope(semantics, format!("{}-fish-{}", semantics.as_str(), idx)));
            }
        });

        EnvelopeList::new_unwrap(envelopes)
    }

    #[test]
    fn create_proper_block_index() {
        let foo_sem = &semantics!("foo");
        let bar_sem = &semantics!("bar");
        let envelopes = create_envelopes(vec![foo_sem, bar_sem], 3);
        let index = BlockIndex::new(&envelopes);
        let foo_fishes = index.0.get(foo_sem).expect("Foo fishes");
        assert_eq!(3, foo_fishes.len());
        for (i, fish_name) in foo_fishes.iter().enumerate().take(3) {
            assert_eq!(&FishName::new(format!("foo-fish-{}", i)).unwrap(), fish_name);
        }
        let bar_fishes = index.0.get(bar_sem).expect("Bar fishes");
        assert_eq!(3, bar_fishes.len());
        for (i, fish_name) in bar_fishes.iter().enumerate().take(3) {
            assert_eq!(&FishName::new(format!("bar-fish-{}", i)).unwrap(), fish_name);
        }
    }

    #[test]
    fn libipld_deser_consnode() -> anyhow::Result<()> {
        let data = hex::decode("a5636d696e191799636d617819179965626c6f636bd82a582500017112209beaabfd04b33edcdebb575ff0e8d10c7b167c80e06a1d600f42b1c74d28d3376470726576d82a582500017112201c1034cd3c886050dc495a233e60c4ee9212457fefd98d773ecd549e94151ed165696e646578a172656467652e61782e73662e6d6574726963738172656467652e61782e73662e6d657472696373")?;
        let _cons_node: ConsNode = DagCborCodec.decode(&data)?;
        Ok(())
    }

    #[test]
    fn libipld_deser_consnode2() -> anyhow::Result<()> {
        let data = hex::decode("a6636d696e190820636d617819082065626c6f636bd82a582500017112201267612e8eea4a3688c4e54e72a00f7da1d1c6f4152cf496bdce6d3e191f3d016470726576d82a582500017112207c56eeab6af97caffc8c5e1d6d68ae2566e3e2290721d7354c9040e337c2921a65696e646578a1635f745f81635f745f68746167496e6465788281781f73656d616e746963733a6374612e696e7075744d6174657269616c46697368818100")?;
        let _cons_node: ConsNode = DagCborCodec.decode(&data)?;
        Ok(())
    }
}
