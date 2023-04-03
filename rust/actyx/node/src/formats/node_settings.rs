use actyx_sdk::language::TagExpr;
use api::formats::Licensing;
use crypto::PublicKey;
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use util::formats::LogSeverity;

// These type definitions need to be kept in sync with the Actyx
// node schema, as found in [0].
// There is a somewhat simple test case in here to make sure, that
// it's mostly in sync, but subtle bugs may be introduced by
// changing the schema w/o changing the types here.

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Events {
    pub read_only: bool,
    #[serde(rename = "_internal")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<serde_json::Value>,
}
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Swarm {
    pub swarm_key: String,
    // TODO: use multiaddr
    pub initial_peers: BTreeSet<String>,
    pub announce_addresses: BTreeSet<String>,
    pub topic: String,
    pub block_cache_size: u64,
    pub block_cache_count: u64,
    pub block_gc_interval: u64,
    pub metrics_interval: u64,
    pub ping_timeout: u64,
    pub bitswap_timeout: u64,
    pub mdns: bool,
    pub branch_cache_size: u64,
    pub gossip_interval: u64,
    pub detection_cycles_low_latency: f64,
    pub detection_cycles_high_latency: f64,
}
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Admin {
    pub display_name: String,
    pub authorized_users: Vec<PublicKey>,
    pub log_levels: LogLevels,
}
#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Api {
    pub events: Events,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogLevels {
    pub node: LogSeverity,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum StreamSize {
    Byte(u64),
    KiloByte(u64),
    MegaByte(u64),
    GigaByte(u64),
}

impl From<u64> for StreamSize {
    /// Assumes that the value is in bytes.
    fn from(b: u64) -> Self {
        StreamSize::Byte(b)
    }
}

impl Into<u64> for StreamSize {
    fn into(self) -> u64 {
        match self {
            StreamSize::Byte(v) => v,
            StreamSize::KiloByte(v) => v * 1000,
            StreamSize::MegaByte(v) => v * 1000 * 1000,
            StreamSize::GigaByte(v) => v * 1000 * 1000 * 1000,
        }
    }
}

impl FromStr for StreamSize {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new("([1-9][0-9]*)(B|kB|MB|GB)?").unwrap();
        }
        let captures = RE.captures(s).ok_or(anyhow::anyhow!("Failed to parse string."))?;
        let value = captures.get(0).map(|v| v.as_str()).unwrap_or("0").parse::<u64>()?;
        let unit = captures.get(1).map(|u| u.as_str());
        Ok(match unit {
            None | Some("B") => Self::Byte(value),
            Some("kB") => Self::KiloByte(value),
            Some("MB") => Self::MegaByte(value),
            Some("GB") => Self::GigaByte(value),
            _ => unreachable!("This should've been covered by the regex."),
        })
    }
}

#[cfg(test)]
mod test_stream_size {
    use std::str::FromStr;

    use crate::node_settings::StreamSize;

    #[test]
    fn test_from_kb() {
        assert_eq!(StreamSize::from_str("1kB").unwrap(), StreamSize::KiloByte(1));
        assert_eq!(StreamSize::from_str("1190kB").unwrap(), StreamSize::KiloByte(1190));
        assert_eq!(
            StreamSize::from_str("9340123kB").unwrap(),
            StreamSize::KiloByte(9340123)
        );
    }

    #[test]
    fn test_from_mb() {
        assert_eq!(StreamSize::from_str("1MB").unwrap(), StreamSize::MegaByte(1));
        assert_eq!(StreamSize::from_str("1190MB").unwrap(), StreamSize::MegaByte(1190));
        assert_eq!(
            StreamSize::from_str("9340123MB").unwrap(),
            StreamSize::MegaByte(9340123)
        );
    }

    #[test]
    fn test_from_gb() {
        assert_eq!(StreamSize::from_str("1GB").unwrap(), StreamSize::GigaByte(1));
        assert_eq!(StreamSize::from_str("1190GB").unwrap(), StreamSize::GigaByte(1190));
        assert_eq!(
            StreamSize::from_str("9340123GB").unwrap(),
            StreamSize::GigaByte(9340123)
        );
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum StreamAge {
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
    Days(u64),
}

impl FromStr for StreamAge {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref RE: Regex = Regex::new("([1-9][0-9]*)(s|m|h|D)").unwrap();
        }
        let captures = RE.captures(s).ok_or(anyhow::anyhow!("Failed to parse string."))?;
        let value = captures.get(0).map(|v| v.as_str()).unwrap_or("0").parse::<u64>()?;
        let unit = captures.get(1).map(|u| u.as_str()).unwrap_or("s");
        Ok(match unit {
            "s" => Self::Seconds(value),
            "m" => Self::Minutes(value),
            "h" => Self::Hours(value),
            "D" => Self::Days(value),
            _ => unreachable!("This should've been covered by the regex."),
        })
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Stream {
    /// Number of maximum events to keep
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_events: Option<u64>,
    /// Maximum size (in bytes) the stream occupy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<StreamSize>,
    /// Maximum event age (in seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<StreamAge>,
}

mod tag_expr {
    use std::str::FromStr;

    use actyx_sdk::language::TagExpr;
    use serde::{de::Visitor, Deserializer, Serializer};

    pub fn serialize<S>(value: &TagExpr, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    struct TagExprVisitor;

    impl<'de> Visitor<'de> for TagExprVisitor {
        type Value = TagExpr;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string containing a valid AQL tag expression.")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            TagExpr::from_str(v).map_err(E::custom)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<TagExpr, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(TagExprVisitor)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug)]
pub struct Route {
    #[serde(with = "tag_expr")]
    pub from: TagExpr,
    pub into: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Debug, Default)]
pub struct EventRouting {
    pub streams: BTreeMap<String, swarm::RetainConfig>,
    pub routes: Vec<Route>,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub swarm: Swarm,
    pub admin: Admin,
    pub licensing: Licensing,
    pub api: Api,
    pub event_routing: EventRouting,
}

impl Settings {
    #[cfg(test)]
    pub fn sample() -> Self {
        use maplit::btreeset;
        Self {
            swarm: Swarm {
                swarm_key: "abcd".to_string(),
                initial_peers: btreeset!["some bootstrap node".into()],
                announce_addresses: btreeset![],
                topic: "some topic".into(),
                block_cache_count: 1024 * 128,
                block_cache_size: 1024 * 1024 * 1024,
                block_gc_interval: 300,
                metrics_interval: 1800,
                ping_timeout: 5,
                bitswap_timeout: 15,
                mdns: true,
                branch_cache_size: 67108864,
                gossip_interval: 10,
                detection_cycles_low_latency: 2.0,
                detection_cycles_high_latency: 5.0,
            },
            admin: Admin {
                display_name: "some name".into(),
                log_levels: LogLevels::default(),
                authorized_users: vec![],
            },
            licensing: Licensing::default(),
            api: Api {
                events: Events {
                    internal: None,
                    read_only: true,
                },
            },
            event_routing: Default::default(),
        }
    }
}
