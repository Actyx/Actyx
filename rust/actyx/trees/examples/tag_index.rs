use actyxos_sdk::{tags, Tag};
use cbor_tag_index::TagIndex;
use libipld::{cbor::DagCbor, cbor::DagCborCodec, codec::Codec, Ipld};
use std::str::FromStr;

fn compresss_zstd_cbor<T: DagCbor>(value: &T) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    let cbor = DagCborCodec.encode(value)?;
    let mut compressed: Vec<u8> = Vec::new();
    zstd::stream::copy_encode(std::io::Cursor::new(cbor), &mut compressed, 10)?;
    Ok(compressed)
}

fn decompress_zstd_cbor<T: DagCbor>(compressed: &[u8]) -> std::result::Result<T, Box<dyn std::error::Error>> {
    let mut decompressed: Vec<u8> = Vec::new();
    zstd::stream::copy_decode(compressed, &mut decompressed)?;
    Ok(DagCborCodec.decode(&decompressed)?)
}

fn main() -> anyhow::Result<()> {
    let tags = (0..5000)
        .map(|i| {
            let fizz = i % 3 == 0;
            let buzz = i % 5 == 0;
            if fizz && buzz {
                tags! {"fizzbuzz", "com.somecompany.somenamespace.someapp.sometype"}
            } else if fizz {
                tags! {"fizz", "org.schema.registry.someothertype"}
            } else if buzz {
                tags! {"buzz", "factory.provider.interface.adapter"}
            } else {
                let tag = Tag::from_str(&*format!("{}", i % 11)).unwrap();
                tags! { tag, "we.like.long.identifiers.because.they.seem.professional" }
            }
        })
        .collect::<Vec<_>>();
    let large = TagIndex::new(tags.clone()).unwrap();
    let compressed = compresss_zstd_cbor(&large).unwrap();
    let large1: TagIndex<Tag> = decompress_zstd_cbor(&compressed).unwrap();
    assert_eq!(large, large1);
    println!("naive cbor {}", DagCborCodec.encode(&tags).unwrap().len());
    println!("index cbor {}", DagCborCodec.encode(&large).unwrap().len());
    println!("compressed {}", compressed.len());

    let index = TagIndex::new(vec![tags!("a"), tags!("a", "b"), tags!("a"), tags!("a", "b")])?;
    let cbor = DagCborCodec.encode(&index)?;
    let text: Ipld = DagCborCodec.decode(&cbor)?;
    println!("{:?}", index);
    println!("{:?}", text);

    // TODO run queries
    Ok(())
}
