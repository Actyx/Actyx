use crate::cmd;
use anyhow::Result;
use async_trait::async_trait;
use clap::{App, Arg, ArgMatches, SubCommand};
use futures::StreamExt;
use serde_value::Value as SValue;
use std::default::Default;
use swarm::BanyanStore;

pub struct Cmd;

pub fn args() -> App<'static, 'static> {
    SubCommand::with_name("monitorPubsub")
        .about("Show plain text of a pubsub topic")
        .arg(
            Arg::with_name("TOPIC")
                .help("Topic to listen on")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("pretty")
                .help("pretty print json")
                .long("pretty")
                .short("p")
                .required(false),
        )
}

///
/// Replace all blobs in this serde data model object with base64 strings
///
fn replace_blobs_with_hex(value: &mut SValue) {
    match value {
        SValue::Newtype(elem) => replace_blobs_with_hex(elem),
        SValue::Option(Some(option)) => replace_blobs_with_hex(option.as_mut()),
        SValue::Seq(elems) => {
            for elem in elems {
                replace_blobs_with_hex(elem)
            }
        }
        SValue::Map(map) => {
            let mut temp: std::collections::BTreeMap<SValue, SValue> = Default::default();
            std::mem::swap(map, &mut temp);
            for (mut k, mut v) in temp.into_iter() {
                replace_blobs_with_hex(&mut k);
                replace_blobs_with_hex(&mut v);
                map.insert(k, v);
            }
        }
        SValue::Bytes(bytes) => *value = SValue::String(hex::encode(bytes)),
        _ => {}
    }
}

#[async_trait]
impl cmd::Command for Cmd {
    fn name(&self) -> &str {
        "monitorPubsub"
    }

    async fn run(&self, matches: &ArgMatches<'_>, store: BanyanStore) -> Result<()> {
        let topic = String::from(matches.value_of("TOPIC").expect("Topic is mandatory"));
        let pretty = matches.is_present("pretty");
        let client = store.ipfs();
        let mut stream = client.subscribe(&topic).unwrap();
        while let Some(msg) = stream.next().await {
            let mut msg = match util::serde_util::from_json_or_cbor_slice::<serde_value::Value>(msg.as_slice()) {
                Ok(msg) => msg,
                Err(err) => {
                    eprintln!("Error reading from ipfs topic {}: {}", topic, err);
                    continue;
                }
            };
            replace_blobs_with_hex(&mut msg);
            let text = if pretty {
                serde_json::to_string_pretty(&msg)
            } else {
                serde_json::to_string(&msg)
            }
            .unwrap();
            println!("{}", text)
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maplit::*;

    #[test]
    fn test_value_transform() {
        let mut map = SValue::Map(btreemap! {
            SValue::Bytes(b"seq".to_vec()) => SValue::Seq(vec![SValue::Bytes(b"bytes".to_vec())]),
            SValue::Bytes(b"option".to_vec()) => SValue::Option(Some(Box::new(SValue::Bytes(b"bytes".to_vec())))),
            SValue::Bytes(b"newtype".to_vec()) => SValue::Newtype(Box::new(SValue::Bytes(b"bytes".to_vec()))),
            SValue::Bytes(b"string".to_vec()) => SValue::String("unchanged".into()),
        });
        replace_blobs_with_hex(&mut map);
        let res = serde_json::to_string(&map).unwrap();
        assert_eq!(
            res,
            r#"{"6e657774797065":"6279746573","6f7074696f6e":"6279746573","736571":["6279746573"],"737472696e67":"unchanged"}"#
        )
    }
}
