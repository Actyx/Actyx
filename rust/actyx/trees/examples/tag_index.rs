use actyxos_sdk::{tags, Tag};
use trees::tag_index::*;

use serde::{de::DeserializeOwned, Serialize};
use std::str::FromStr;

fn compresss_zstd_cbor<T: Serialize>(value: &T) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    let cbor = serde_cbor::to_vec(&value)?;
    let mut compressed: Vec<u8> = Vec::new();
    zstd::stream::copy_encode(std::io::Cursor::new(cbor), &mut compressed, 10)?;
    Ok(compressed)
}

fn decompress_zstd_cbor<T: DeserializeOwned>(compressed: &[u8]) -> std::result::Result<T, Box<dyn std::error::Error>> {
    let mut decompressed: Vec<u8> = Vec::new();
    zstd::stream::copy_decode(compressed, &mut decompressed)?;
    Ok(serde_cbor::from_slice(&decompressed)?)
}

fn main() {
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
    let large = TagIndex::from_elements(&tags);
    let compressed = compresss_zstd_cbor(&large).unwrap();
    let large1: TagIndex = decompress_zstd_cbor(&compressed).unwrap();
    assert_eq!(large, large1);
    println!("naive cbor {}", serde_cbor::to_vec(&tags).unwrap().len());
    println!("index cbor {}", serde_cbor::to_vec(&large).unwrap().len());
    println!("compressed {}", compressed.len());

    let index = TagIndex::from_elements(&[tags!("a"), tags!("a", "b"), tags!("a"), tags!("a", "b")]);
    let text = serde_json::to_string(&index).unwrap();
    println!("{:?}", index);
    println!("{}", text);

    // TODO run queries
}
