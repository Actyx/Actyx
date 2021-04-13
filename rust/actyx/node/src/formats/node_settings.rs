use actyxos_sdk::AppId;
use ax_config::StoreConfig;
use crypto::PublicKey;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
/// These type definitions need to be kept in sync with the ActyxOS
/// node schema, as found in [0].
/// There is a somewhat simple test case in here to make sure, that
/// it's mostly in sync, but subtle bugs may be introduced by
/// changing the schema w/o changing the types here.
use util::formats::{ActyxOSResult, ActyxOSResultExt, LogSeverity};
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Licensing {
    pub node: String,
    pub apps: BTreeMap<AppId, String>,
}
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
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
    pub bootstrap_nodes: BTreeSet<String>,
    pub announce_addresses: BTreeSet<String>,
    pub topic: String,
}
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Admin {
    pub display_name: String,
    pub authorized_users: Vec<PublicKey>,
    pub log_levels: LogLevels,
}
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Api {
    pub events: Events,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogLevels {
    pub node: LogSeverity,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub swarm: Swarm,
    pub admin: Admin,
    pub licensing: Licensing,
    pub api: Api,
}

impl Settings {
    #[cfg(test)]
    pub fn sample() -> Self {
        use maplit::btreeset;
        Self {
          swarm: Swarm {
            swarm_key: "L2tleS9zd2FybS9wc2svMS4wLjAvCi9iYXNlMTYvCjY1YjM1NDhjYTg0YWZmMTkwZjlkYTkzZThkMjQ2YWU1NjU5ZDJlZGQ1M2ZjNjQ4MjdiOWM0NTdmNWY4MzAyNGIK".into(),
            bootstrap_nodes: btreeset!["some bootstrap node".into()],
            announce_addresses: btreeset![],
            topic: "some topic".into(),
          },
          admin: Admin {
            display_name: "some name".into(),
            log_levels: LogLevels::default(),
            authorized_users: vec![],
          },
          licensing: Licensing {
              node: "development".into(),
              apps: BTreeMap::default(),
          },
          api: Api {
              events: Events {
                  internal: None,
                  read_only: true,
              },
          },
      }
    }
    pub fn store_config<P: AsRef<std::path::Path>>(&self, working_dir: P) -> ActyxOSResult<StoreConfig> {
        let events_config = &self.api.events;
        let swarm = &self.swarm;

        let mut config = if let Some(internal) = &events_config.internal {
            StoreConfig::from_json(&serde_json::to_string(internal).unwrap()).ax_internal()?
        } else {
            StoreConfig::new(swarm.topic.clone())
        };
        let sanitized_topic = swarm.topic.replace('/', "_");
        config.db_path = Some(working_dir.as_ref().join(format!("{}.sqlite", sanitized_topic)));
        config.ipfs_node.enable_publish = !events_config.read_only;
        config.ipfs_node.db_path = Some(working_dir.as_ref().join(format!("{}-blocks.sqlite", sanitized_topic)));
        config.ipfs_node.listen = vec!["/ip4/0.0.0.0/tcp/4001".parse().unwrap()];
        config.ipfs_node.bootstrap = swarm
            .bootstrap_nodes
            .iter()
            .map(|s| s.parse().ax_internal())
            .collect::<ActyxOSResult<_>>()?;
        config.ipfs_node.pre_shared_key = Some(swarm.swarm_key.clone());
        config.ipfs_node.external_addresses = swarm
            .announce_addresses
            .iter()
            .map(|s| s.parse().ax_internal())
            .collect::<ActyxOSResult<_>>()?;
        Ok(config)
    }
}
