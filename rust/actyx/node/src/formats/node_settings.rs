use actyx_sdk::language::TagExpr;
use api::formats::Licensing;
use crypto::PublicKey;
use serde::{Deserialize, Deserializer, Serialize};
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
    #[serde(deserialize_with = "deserialize_authorized_users")]
    pub authorized_users: Vec<PublicKey>,
    pub log_levels: LogLevels,
}

fn deserialize_authorized_users<'de, D>(deserializer: D) -> Result<Vec<PublicKey>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde_json::Value;
    let maybe_array = Value::deserialize(deserializer)?;

    if let Value::Array(items) = maybe_array {
        let iter_res = items.into_iter().map(serde_json::from_value::<PublicKey>);

        let index_of_errors = iter_res
            .clone()
            .enumerate()
            .filter(|(_, item)| item.is_err())
            .map(|(index, _)| index.to_string())
            .collect::<Vec<_>>()
            .join(",");

        tracing::warn!(
            "found invalid entries in config/admin/authorizedUsers at index: {}",
            index_of_errors
        );

        Ok(iter_res.filter_map(|x| x.ok()).collect())
    } else {
        Err(serde::de::Error::custom("Expected an array of string"))
    }
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

#[cfg(test)]
mod tests {

    #[test]
    pub fn sample_with_invalid_authorized_users() {
        use super::Admin;
        use super::Settings;
        let mut sample_json = serde_json::to_value(Settings::sample().admin).unwrap();
        if let serde_json::Value::Object(admin_settings) = &mut sample_json {
            let authorized_users = admin_settings.get_mut("authorizedUsers").unwrap();
            if let serde_json::Value::Array(authorized_users_as_array) = authorized_users {
                // valid
                authorized_users_as_array.push("0BvjSPuvSFnxeJu+PWfFtZBpnfcrjh6pcz1e6kQjxNhg=".into());
                authorized_users_as_array.push("0OAapA3dk0KzFVJrEEYwvP3CLKY/UEYImE+B8oV+19EU=".into());
                // invalid
                authorized_users_as_array.push("0FtjBTIiGoM3LlS4xJcFnUxkPItCBWWlOmNnJgmTtTLQ=".into());
            }
        }

        let admin = serde_json::from_str::<Admin>(sample_json.to_string().as_str()).unwrap();
        assert_eq!(admin.authorized_users.len(), 2);
    }
}
