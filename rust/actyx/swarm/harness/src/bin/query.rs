#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use actyx_sdk::{
        app_id,
        service::{EventService, Order, PublishEvent, PublishRequest, QueryRequest, QueryResponse},
        tags, AppManifest, OffsetMap, Payload,
    };
    use anyhow::Context;
    use async_std::{future::timeout, task::sleep};
    use futures::{future, StreamExt};
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        time::Duration,
    };
    use structopt::StructOpt;
    use swarm_cli::Event;
    use swarm_harness::{api::Api, fully_meshed, HarnessOpts};

    fn make_events(n: usize) -> Vec<PublishEvent> {
        (0..n)
            .map(|i| PublishEvent {
                tags: tags!("a", "b"),
                payload: Payload::from_json_str(&*format!("{}", i)).unwrap(),
            })
            .collect()
    }

    let app_manifest = AppManifest {
        app_id: app_id!("com.example.query"),
        display_name: "Query test".into(),
        version: "0.1.0".into(),
        signature: None,
    };

    const N: usize = 1000;

    let mut opts = HarnessOpts::from_args();
    opts.enable_api = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 30001));
    opts.enable_fast_path = true;
    opts.enable_slow_path = true;
    opts.enable_root_map = true;
    opts.enable_discovery = true;
    opts.n_bootstrap = 1;

    swarm_harness::setup_env()?;
    swarm_harness::run_netsim(opts, move |mut sim| async move {
        let api = Api::new(&mut sim, app_manifest)?;

        for machine in sim.machines() {
            api.run(machine.id(), |api| async move {
                api.publish(PublishRequest { data: make_events(N) }).await?;

                let upper_bound = api.offsets().await?.present;
                let count = (&upper_bound - &OffsetMap::default()) as usize;
                assert!(count >= N);

                let result = api
                    .query(QueryRequest {
                        lower_bound: None,
                        upper_bound: Some(upper_bound),
                        query: "FROM allEvents".parse()?,
                        order: Order::Asc,
                    })
                    .await?
                    .filter(|resp| future::ready(matches!(resp, QueryResponse::Event(_))))
                    .collect::<Vec<_>>()
                    .await;

                assert_eq!(result.len(), count);
                Ok(result)
            })
            .await?;
        }

        fully_meshed::<Event>(&mut sim, Duration::from_secs(60)).await?;

        let expected = N * sim.machines().len();
        let mut tries = 10i32;

        while tries > 0 {
            sleep(Duration::from_secs(2)).await;

            // just dump events for logging purposes
            for machine in sim.machines_mut() {
                for ev in machine.drain() {
                    tracing::info!("{} got ev {:?}", machine.id(), ev);
                }
            }

            let mut not_yet = false;
            for machine in sim.machines() {
                let count = api
                    .run(machine.id(), |api| async move {
                        let upper_bound = api.offsets().await?.present;
                        let count = (&upper_bound - &OffsetMap::default()) as usize;

                        let result = timeout(
                            Duration::from_secs(10),
                            api.query(QueryRequest {
                                lower_bound: None,
                                upper_bound: Some(upper_bound),
                                query: "FROM allEvents".parse()?,
                                order: Order::Asc,
                            })
                            .await?
                            .filter(|resp| future::ready(matches!(resp, QueryResponse::Event(_))))
                            .collect::<Vec<_>>(),
                        )
                        .await
                        .with_context(|| format!("query for {} timed out", machine.id()))?;

                        assert_eq!(result.len(), count);
                        tracing::info!("{} got {} events", machine.id(), count);
                        Ok(count)
                    })
                    .await?;
                not_yet |= count < expected;
            }
            if not_yet {
                tries -= 1;
            } else {
                break;
            }
        }

        Ok(())
    })
}

#[cfg(not(target_os = "linux"))]
fn main() {}
