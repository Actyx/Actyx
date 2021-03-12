#[test]
fn default_settings() {
    let schema = serde_json::from_reader(
        std::fs::File::open("../../../protocols/json-schema/os/node-settings.schema.json").unwrap(),
    )
    .unwrap();
    let json = axossettings::Validator::new(schema)
        .unwrap()
        .validate_with_defaults(None, &axossettings::Scope::root())
        .unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "general": {
              "swarmKey": "L2tleS9zd2FybS9wc2svMS4wLjAvCi9iYXNlMTYvCmQ3YjBmNDFjY2ZlYTEyM2FkYTJhYWI0MmY2NjRjOWUyNWUwZWYyZThmNGJjNjJlOTg3NmE3NDU1MTc3ZWQzOGIK",
              "bootstrapNodes": [],
              "announceAddresses": [],
              "displayName": "Default Node",
              "logLevels": {
                "os": "INFO",
                "apps": "INFO"
              },
              "authorizedKeys": []
            },
            "licensing": {
              "os": "development",
              "apps": {}
            },
            "services": {
              "eventService": {
                "topic": "default-topic",
                "readOnly": false
              }
            }
          }
        )
    );

    use actyxos_lib::LogSeverity::*;
    use maplit::{btreemap, btreeset};
    use node::os_settings::*;
    let settings: Settings = serde_json::from_value(json).unwrap();
    assert_eq!(
      settings,
      Settings {
        general: General {
          swarm_key: "L2tleS9zd2FybS9wc2svMS4wLjAvCi9iYXNlMTYvCmQ3YjBmNDFjY2ZlYTEyM2FkYTJhYWI0MmY2NjRjOWUyNWUwZWYyZThmNGJjNjJlOTg3NmE3NDU1MTc3ZWQzOGIK".to_string(),
          bootstrap_nodes: btreeset! {},
          announce_addresses: btreeset! {},
          display_name: "Default Node".to_string(),
          log_levels: LogLevels {
            os: Info,
            apps: Info,
          },
          authorized_keys: vec![]
        },
        licensing: Licensing {
          os: "development".to_string(),
          apps: btreemap! {},
        },
        services: Services {
          event_service: EventService {
            topic: "default-topic".to_string(),
            read_only: false,
            internal: None,
          }
        }
      }
    );

    use ax_config::{IpfsNodeConfig, StoreConfig};
    use std::time::Duration;
    let store_config = settings.store_config(std::path::PathBuf::default()).unwrap();
    assert_eq!(
      store_config,
      StoreConfig {
        topic: "default-topic".to_string(),
        monitoring_topic: "default-topic/monitoring".to_string(),
        api_addr: "/ip4/127.0.0.1/tcp/4454".parse().unwrap(),
        ipfs_request_parallelism: 10,
        block_cache: 50000000,
        cons_cache: 4096,
        emit_interval: Duration::from_millis(500),
        validation_attempts: 5,
        db_path: Some("default-topic.sqlite".into()),
        compaction_schedule: Duration::from_millis(3600000),
        number_of_threads: None,
        gossip_interval: Duration::from_millis(30000),
        log_to_pubsub: true,
        log_verbosity: Info,
        ipfs_node: IpfsNodeConfig {
          bootstrap: vec![],
          listen: vec!["/ip4/0.0.0.0/tcp/4001".parse().unwrap()],
          external_addresses: vec![],
          pre_shared_key: Some("L2tleS9zd2FybS9wc2svMS4wLjAvCi9iYXNlMTYvCmQ3YjBmNDFjY2ZlYTEyM2FkYTJhYWI0MmY2NjRjOWUyNWUwZWYyZThmNGJjNjJlOTg3NmE3NDU1MTc3ZWQzOGIK".to_string()),
          db_path: Some("default-topic-blocks.sqlite".into()),
          db_size: None,
          enable_mdns: true,
          enable_publish: true,
          identity: None,
        }
      }
    );
}
