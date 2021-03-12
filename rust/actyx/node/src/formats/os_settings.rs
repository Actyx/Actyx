/// These type definitions need to be kept in sync with the ActyxOS
/// node schema, as found in [0].
/// There is a somewhat simple test case in here to make sure, that
/// it's mostly in sync, but subtle bugs may be introduced by
/// changing the schema w/o changing the types here.
use actyxos_lib::{
    formats::{ActyxOSResultExt, AppId},
    ActyxOSResult, LogSeverity,
};
use ax_config::StoreConfig;
use crypto::PublicKey;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Licensing {
    pub os: String,
    pub apps: BTreeMap<AppId, String>,
}
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EventService {
    pub topic: String,
    pub read_only: bool,
    #[serde(rename = "_internal")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<serde_json::Value>,
}
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct General {
    pub swarm_key: String,
    // TODO: use multiaddr
    pub bootstrap_nodes: BTreeSet<String>,
    pub announce_addresses: BTreeSet<String>,
    pub display_name: String,
    pub log_levels: LogLevels,
    pub authorized_keys: Vec<PublicKey>,
}
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Services {
    pub event_service: EventService,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default, Copy)]
#[serde(rename_all = "camelCase")]
pub struct LogLevels {
    pub os: LogSeverity,
    pub apps: LogSeverity,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub general: General,
    pub licensing: Licensing,
    pub services: Services,
}

impl Settings {
    #[cfg(test)]
    pub fn sample() -> Self {
        use maplit::btreeset;
        Self {
        general: General {
            display_name: "some name".into(),
            bootstrap_nodes: btreeset!["some bootstrap node".into()],
            swarm_key: "L2tleS9zd2FybS9wc2svMS4wLjAvCi9iYXNlMTYvCjY1YjM1NDhjYTg0YWZmMTkwZjlkYTkzZThkMjQ2YWU1NjU5ZDJlZGQ1M2ZjNjQ4MjdiOWM0NTdmNWY4MzAyNGIK".into(),
            announce_addresses: btreeset![],
            log_levels: LogLevels::default(),
            authorized_keys: vec![],
        },
        licensing: Licensing {
            os: "development".into(),
            apps: BTreeMap::default(),
        },
        services: Services {
            event_service: EventService {
                internal: None,
                read_only: true,
                topic: "some topic".into(),
            },
        },
    }
    }
    pub fn store_config<P: AsRef<std::path::Path>>(&self, working_dir: P) -> ActyxOSResult<StoreConfig> {
        let eventservice_config = &self.services.event_service;
        let general = &self.general;

        let mut config = if let Some(internal) = &eventservice_config.internal {
            StoreConfig::from_json(&serde_json::to_string(internal).unwrap()).ax_internal()?
        } else {
            StoreConfig::new(eventservice_config.topic.clone())
        };
        let sanitized_topic = eventservice_config.topic.replace('/', "_");
        config.db_path = Some(working_dir.as_ref().join(format!("{}.sqlite", sanitized_topic)));
        config.ipfs_node.enable_publish = !eventservice_config.read_only;
        config.ipfs_node.db_path = Some(working_dir.as_ref().join(format!("{}-blocks.sqlite", sanitized_topic)));
        config.ipfs_node.listen = vec!["/ip4/0.0.0.0/tcp/4001".parse().unwrap()];
        config.ipfs_node.bootstrap = general
            .bootstrap_nodes
            .iter()
            .map(|s| s.parse().ax_internal())
            .collect::<ActyxOSResult<_>>()?;
        config.ipfs_node.pre_shared_key = Some(general.swarm_key.clone());
        config.ipfs_node.external_addresses = general
            .announce_addresses
            .iter()
            .map(|s| s.parse().ax_internal())
            .collect::<ActyxOSResult<_>>()?;
        Ok(config)
    }
}
