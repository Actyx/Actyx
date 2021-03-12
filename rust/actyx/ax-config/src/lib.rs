use actyxos_lib::LogSeverity;
use anyhow::Result;
use parity_multiaddr::Multiaddr;
use serde::Deserialize;
use serde_with::{serde_as, DurationMilliSeconds};
use std::path::PathBuf;
use std::time::Duration;
use util::SocketAddrHelper;

#[serde_as]
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct StoreConfig {
    pub topic: String,
    /// Defaults to "{topic}/monitoring".
    #[serde(default)]
    pub monitoring_topic: String,
    /// Bind address for the api.
    #[serde(default = "defaults::api_addr")]
    pub api_addr: SocketAddrHelper,
    ///
    #[serde(default = "defaults::ipfs_request_parallelism")]
    pub ipfs_request_parallelism: usize,
    ///
    #[serde(default = "defaults::block_cache")]
    pub block_cache: usize,
    ///
    #[serde(default = "defaults::cons_cache")]
    pub cons_cache: usize,
    ///
    #[serde(default = "defaults::validation_attempts")]
    pub validation_attempts: usize,
    ///
    #[serde(default = "defaults::emit_interval")]
    #[serde(rename = "emit_interval_ms")]
    #[serde_as(as = "DurationMilliSeconds")]
    pub emit_interval: Duration,
    ///
    #[serde(default = "defaults::compaction_schedule")]
    #[serde(rename = "compaction_schedule_ms")]
    #[serde_as(as = "DurationMilliSeconds")]
    pub compaction_schedule: Duration,
    ///
    #[serde(default = "defaults::gossip_interval")]
    #[serde(rename = "gossip_interval_ms")]
    #[serde_as(as = "DurationMilliSeconds")]
    pub gossip_interval: Duration,
    ///
    #[serde(default)]
    pub db_path: Option<PathBuf>,
    /// Override for tokio's default of core_threads = hardware threads
    #[serde(default)]
    pub number_of_threads: Option<usize>,
    ///
    #[serde(default = "defaults::log_to_pubsub")]
    pub log_to_pubsub: bool,
    ///
    #[serde(default = "defaults::log_verbosity")]
    pub log_verbosity: LogSeverity,
    /// config for IPFS full node
    #[serde(default)]
    pub ipfs_node: IpfsNodeConfig,
}

impl StoreConfig {
    pub fn new(topic: String) -> Self {
        let monitoring_topic = format!("{}/monitoring", &topic);
        Self {
            topic,
            monitoring_topic,
            api_addr: defaults::api_addr(),
            ipfs_request_parallelism: defaults::ipfs_request_parallelism(),
            block_cache: defaults::block_cache(),
            cons_cache: defaults::cons_cache(),
            validation_attempts: defaults::validation_attempts(),
            emit_interval: defaults::emit_interval(),
            compaction_schedule: defaults::compaction_schedule(),
            gossip_interval: defaults::gossip_interval(),
            db_path: Default::default(),
            number_of_threads: Default::default(),
            log_to_pubsub: defaults::log_to_pubsub(),
            log_verbosity: defaults::log_verbosity(),
            ipfs_node: Default::default(),
        }
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let mut config: Self = serde_json::from_str(json)?;
        if config.monitoring_topic.is_empty() {
            config.monitoring_topic = format!("{}/monitoring", &config.topic);
        }
        Ok(config)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct IpfsNodeConfig {
    /// Static multiaddrs to bootstrap the swarm.
    #[serde(default)]
    pub bootstrap: Vec<Multiaddr>,
    /// Addresses the swarm will listen on for incoming connections.
    #[serde(default = "defaults::ipfs_node::listen")]
    pub listen: Vec<Multiaddr>,
    /// External addresses of the swarm.
    #[serde(default)]
    pub external_addresses: Vec<Multiaddr>,
    /// Optional pre-shared symmetric encryption key for private swarms.
    #[serde(default)]
    pub pre_shared_key: Option<String>,
    /// The path to use for the block store db. If none is provided the block store will use an
    /// in-memory ephemeral db.
    #[serde(default)]
    pub db_path: Option<PathBuf>,
    /// Size of the db in bytes.
    #[serde(default)]
    pub db_size: Option<u64>,
    /// Enables publishing of messages, defaults to `true`.
    #[serde(default = "defaults::ipfs_node::enable_publish")]
    pub enable_publish: bool,
    /// Enables discovering peers with mdns, defaults to `true`.
    #[serde(default = "defaults::ipfs_node::enable_mdns")]
    pub enable_mdns: bool,
    /// Optional key pair to use. If none is provided an ephemeral public key will be generated.
    #[serde(default)]
    pub identity: Option<String>,
}

impl Default for IpfsNodeConfig {
    fn default() -> Self {
        Self {
            bootstrap: Default::default(),
            listen: defaults::ipfs_node::listen(),
            external_addresses: Default::default(),
            pre_shared_key: Default::default(),
            db_path: Default::default(),
            db_size: Default::default(),
            enable_publish: defaults::ipfs_node::enable_publish(),
            enable_mdns: defaults::ipfs_node::enable_mdns(),
            identity: Default::default(),
        }
    }
}

mod defaults {
    use util::SocketAddrHelper;

    use super::{Duration, LogSeverity, Multiaddr};

    pub fn api_addr() -> SocketAddrHelper {
        "/ip4/127.0.0.1/tcp/4454".parse().unwrap()
    }

    pub fn ipfs_request_parallelism() -> usize {
        10
    }

    pub fn block_cache() -> usize {
        50_000_000
    }

    pub fn cons_cache() -> usize {
        4096
    }

    pub fn validation_attempts() -> usize {
        5
    }

    pub fn emit_interval() -> Duration {
        Duration::from_millis(500)
    }

    pub fn compaction_schedule() -> Duration {
        Duration::from_millis(3_600_000)
    }

    pub fn gossip_interval() -> Duration {
        Duration::from_millis(30_000)
    }

    pub fn log_to_pubsub() -> bool {
        true
    }

    pub fn log_verbosity() -> LogSeverity {
        LogSeverity::Info
    }

    pub mod ipfs_node {
        use super::Multiaddr;

        pub fn listen() -> Vec<Multiaddr> {
            vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()]
        }

        pub fn enable_publish() -> bool {
            true
        }

        pub fn enable_mdns() -> bool {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_allow_providing_a_custom_json() {
        let config = r#"{
        "topic": "mars",
        "block_cache": 42,
        "db_path": "jupiter",
        "log_to_pubsub": false,
        "log_verbosity": "error",
        "ipfs_node": {
          "bootstrap": [ "/ip4/127.0.0.1/tcp/4711", "/ip4/127.0.0.1/tcp/4722" ],
          "pre_shared_key": "secret",
          "db_path": "something_else"
        }
        }"#;
        let cfg = StoreConfig::from_json(config).unwrap();
        assert_eq!(
            cfg,
            StoreConfig {
                topic: "mars".to_string(),
                monitoring_topic: "mars/monitoring".to_string(),
                api_addr: "/ip4/127.0.0.1/tcp/4454".parse().unwrap(),
                ipfs_request_parallelism: 10,
                block_cache: 42,
                cons_cache: 4096,
                emit_interval: Duration::from_millis(500),
                compaction_schedule: Duration::from_millis(3_600_000),
                gossip_interval: Duration::from_millis(30_000),
                validation_attempts: 5,
                db_path: Some("jupiter".into()),
                number_of_threads: None,
                log_to_pubsub: false,
                log_verbosity: LogSeverity::Error,
                ipfs_node: IpfsNodeConfig {
                    bootstrap: vec![
                        "/ip4/127.0.0.1/tcp/4711".parse().unwrap(),
                        "/ip4/127.0.0.1/tcp/4722".parse().unwrap(),
                    ],
                    listen: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()],
                    external_addresses: vec![],
                    pre_shared_key: Some("secret".to_string()),
                    db_path: Some("something_else".into()),
                    db_size: None,
                    enable_publish: true,
                    enable_mdns: true,
                    identity: None,
                }
            }
        )
    }
}
