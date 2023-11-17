use axlib::{
    api::licensing::Licensing,
    node::node_settings::*,
    settings::{Repository, Scope},
};

#[test]
fn node_schema_in_sync() {
    use maplit::btreeset;
    let sample_settings = Settings {
        swarm: Swarm {
            initial_peers: btreeset![
                "/ip4/127.0.0.1/tcp/4001/p2p/QmaAxuktPMR3ESHe9Pru8kzzzSGvsUie7UFJPfCWqTzzzz".into()
            ],
            announce_addresses: btreeset![],
            swarm_key: "MDAwMDAwMDAxMTExMTExMTIyMjIyMjIyMzMzMzMzMzM=".into(),
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
    };
    let current_schema: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/resources/json-schema/node-settings.schema.json"
    )))
    .unwrap();

    let repo = Repository::new_in_memory();
    let scope: Scope = "com.actyx".parse().unwrap();
    repo.set_schema(&scope, current_schema).unwrap();
    repo.update_settings(&scope, serde_json::to_value(&sample_settings).unwrap(), false)
        .unwrap();
}
