use actyxos_sdk::{Dnf, Tag, TagSet};
use libipld::cbor::{decode::read_u8, DagCborCodec};
use libipld::codec::{Decode, Encode};
use libipld::error::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::BTreeSet,
    convert::TryFrom,
    io::{Read, Seek, Write},
};
use vec_collections::VecSet;

/// An index set is a set of u32 indices into the string table that will not allocate for up to 4 indices.
/// The size of a non-spilled IndexSet is 32 bytes on 64 bit architectures, so just 8 bytes more than a Vec.
pub type IndexSet = vec_collections::VecSet<[u32; 4]>;

/// a compact index
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagIndex {
    /// the strings table
    pub(crate) tags: TagSet,
    /// indices in these sets are guaranteed to correspond to strings in the strings table
    pub(crate) elements: Vec<IndexSet>,
}

impl Decode<DagCborCodec> for TagIndex {
    fn decode<R: Read + Seek>(_: DagCborCodec, r: &mut R) -> Result<Self, Error> {
        anyhow::ensure!(read_u8(r)? == 0x82);
        let tags: TagSet = Decode::decode(DagCborCodec, r)?;
        let elements: Vec<Vec<u32>> = Decode::decode(DagCborCodec, r)?;
        let elements: Vec<VecSet<[u32; 4]>> = elements.into_iter().map(Into::into).collect::<Vec<_>>();
        for s in &elements {
            for x in s {
                anyhow::ensure!((*x as usize) < tags.len(), "invalid string index");
            }
        }
        Ok(Self { tags, elements })
    }
}

impl Encode<DagCborCodec> for TagIndex {
    fn encode<W: Write>(&self, c: DagCborCodec, w: &mut W) -> Result<(), Error> {
        w.write_all(&[0x82])?;
        self.tags.encode(c, w)?;
        self.elements
            .iter()
            .map(|v| v.iter().copied().collect::<Vec<_>>())
            .collect::<Vec<_>>()
            .encode(c, w)?;
        Ok(())
    }
}

impl Serialize for TagIndex {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        // serialize as a tuple so it is guaranteed that the strings table is before the indices,
        // in case we ever want to write a clever visitor that matches without building an AST
        // of the deserialized result.
        (&self.tags, &self.elements).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TagIndex {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let (tags, elements) = <(TagSet, Vec<IndexSet>)>::deserialize(deserializer)?;
        // ensure valid indices
        for s in &elements {
            for x in s {
                if *x as usize >= tags.len() {
                    return Err(serde::de::Error::custom("invalid string index"));
                }
            }
        }
        Ok(Self { tags, elements })
    }
}

impl TagIndex {
    pub fn all_tags(&self) -> &TagSet {
        &self.tags
    }

    pub fn num_entries(&self) -> usize {
        self.elements.len()
    }

    /// given a query expression in Dnf form, returns all matching indices
    pub fn matching(&self, query: Dnf) -> Vec<usize> {
        // lookup all strings and translate them into indices.
        // if a single index does not match, the query can not match at all.
        // FIXME Dnf should contain Tags
        let lookup = |s: &BTreeSet<String>| -> Option<IndexSet> {
            s.iter()
                .filter_map(|x| Tag::try_from(&**x).ok())
                .map(|t| self.tags.find(&t).map(|x| x as u32))
                .collect::<Option<_>>()
        };
        // translate the query from strings to indices
        let query = query.0.iter().filter_map(lookup).collect::<Vec<_>>();
        // not a single query can possibly match, no need to iterate.
        if query.is_empty() {
            return Vec::new();
        }
        // check the remaining queries
        self.elements
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                if query.iter().any(|x| x.is_subset(e)) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn as_elements(&self) -> Vec<TagSet> {
        self.iter_elements().collect()
    }

    pub fn iter_elements(&self) -> impl Iterator<Item = TagSet> + '_ {
        self.elements
            .iter()
            .map(move |is| is.iter().filter_map(|i| self.tags.get(*i as usize)).collect::<TagSet>())
    }

    pub fn from_elements(e: &[TagSet]) -> Self {
        let mut tags = BTreeSet::new();
        for a in e.iter() {
            tags.extend(a.iter());
        }
        let tags = TagSet::from(tags);
        let elements = e
            .iter()
            .map(|a| a.iter().map(|e| tags.find(&e).unwrap() as u32).collect::<IndexSet>())
            .collect::<Vec<_>>();
        Self { tags, elements }
    }

    pub fn get(&self, index: usize) -> Option<TagSet> {
        self.elements
            .get(index)
            .map(|is| is.iter().filter_map(|i| self.tags.get(*i as usize)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actyxos_sdk::{tags, Expression};
    use quickcheck::quickcheck;

    fn l(x: &str) -> Expression {
        Expression::literal(x.into())
    }

    #[test]
    fn test_matching_1() {
        let index = TagIndex::from_elements(&[tags! {"a"}, tags! {"a", "b"}, tags! {"a"}, tags! {"a", "b"}]);
        let expr = l("a") | l("b");
        assert_eq!(index.matching(expr.dnf()), vec![0, 1, 2, 3]);
        let expr = l("a") & l("b");
        assert_eq!(index.matching(expr.dnf()), vec![1, 3]);
        let expr = l("c") & l("d");
        assert!(index.matching(expr.dnf()).is_empty());
    }

    #[test]
    fn test_matching_2() {
        let index = TagIndex::from_elements(&[tags! {"a", "b"}, tags! {"b", "c"}, tags! {"c", "a"}, tags! {"a", "b"}]);
        let expr = l("a") | l("b") | l("c");
        assert_eq!(index.matching(expr.dnf()), vec![0, 1, 2, 3]);
        let expr = l("a") & l("b");
        assert_eq!(index.matching(expr.dnf()), vec![0, 3]);
        let expr = l("a") & l("b") & l("c");
        assert!(index.matching(expr.dnf()).is_empty());
    }

    #[test]
    fn test_deser_error() {
        // negative index - serde should catch this
        let e1 = r#"[["a","b"],[[0],[0,1],[0],[0,-1]]]"#;
        let x: std::result::Result<TagIndex, _> = serde_json::from_str(e1);
        assert!(x.is_err());

        // index too large - we must catch this in order to uphold the invariants of the index
        let e1 = r#"[["a","b"],[[0],[0,1],[0],[0,2]]]"#;
        let x: std::result::Result<TagIndex, _> = serde_json::from_str(e1);
        assert!(x.is_err());
    }

    quickcheck! {
        fn serde_json_roundtrip(index: TagIndex) -> bool {
            let json = serde_json::to_string(&index).unwrap();
            let index2: TagIndex = serde_json::from_str(&json).unwrap();
            index == index2
        }
    }
}
