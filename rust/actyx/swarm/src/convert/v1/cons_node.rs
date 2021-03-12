use super::{Block, Offset};
use libipld::{cbor::DagCborCodec, raw_value::RawValue, DagCbor, Link};

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
    pub index: Option<RawValue<DagCborCodec>>,

    /// Optional field to indicate that a block is compacted according to a certain compaction algorithm
    #[ipld(rename = "compactedWith")]
    #[ipld(default = None)]
    pub compacted_with: Option<RawValue<DagCborCodec>>,

    /// Optional index data structure containing tags for every index.
    #[ipld(rename = "tagIndex")]
    #[ipld(default = None)]
    pub tag_index: Option<RawValue<DagCborCodec>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use libipld::codec::Codec;

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
