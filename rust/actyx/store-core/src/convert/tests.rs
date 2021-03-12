use std::convert::TryFrom;

use super::*;
use maplit::btreemap;
use tempdir::TempDir;

fn cid(text: &str) -> Cid {
    Cid::from_str(text).unwrap()
}

fn stream(text: &str) -> StreamId {
    let source = SourceId::from_str(text).unwrap();
    StreamId::from(source)
}

#[test]
fn convert_v1() -> anyhow::Result<()> {
    let dir = TempDir::new("convert_from_v1")?;
    let v2_index_path = dir.path().join("db").to_str().expect("illegal file name").to_owned();
    let v2_blocks_path = dir
        .path()
        .join("db-blocks.sqlite")
        .to_str()
        .expect("illegal file name")
        .to_owned();
    convert_from_v1(
        "test-data/v1/_cta_prod_2019-12-16_stripped",
        &v2_index_path,
        ConversionOptions::default(),
    )?;
    let v2_blocks = BlockStore::open(v2_blocks_path, Default::default())?;
    let aliases: Vec<(Vec<u8>, Cid)> = v2_blocks.aliases()?;
    let streams: BTreeMap<StreamId, Cid> = aliases
        .into_iter()
        .filter_map(|(alias, cid)| {
            let alias = StreamAlias::try_from(alias.as_ref()).ok()?;
            let stream = StreamId::try_from(alias).ok()?;
            Some((stream, cid))
        })
        .collect();
    // for (k, v) in &streams {
    //     let s = k.source_id()?;
    //     println!(r#"stream("{}") => cid("{}"),"#, s, v);
    // }
    let expected = btreemap! {
        stream("6Uc7gq28Eat") => cid("bafyreidshd4cmnn3kxvd3gmfpa4xzffpgkvn6crqgdtmr7ua3e6ig6snpq"),
        stream("XgQrd4AoA9R") => cid("bafyreiamwoz7rb4sckuagrp6orxihwatmulane2po3nupfsf3aisn5o454"),
        stream("y9XJiNWir5w") => cid("bafyreie3hmtqoxcxboojfcvub7d2ofo2pgekc3psk6craqa4vq7gfiggi4"),
    };
    assert_eq!(streams.keys().collect::<Vec<_>>(), expected.keys().collect::<Vec<_>>());
    Ok(())
}
