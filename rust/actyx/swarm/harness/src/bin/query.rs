#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use actyxos_sdk::{
        app_id,
        service::{EventService, Order, PublishEvent, PublishRequest, QueryRequest},
        tags, AppManifest, OffsetMap, Payload,
    };
    use async_std::task::sleep;
    use futures::StreamExt;
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        time::Duration,
    };
    use structopt::StructOpt;
    use swarm_cli::{Command, Event};
    use swarm_harness::{api::Api, m, select_single, HarnessOpts, MachineExt};

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
                        upper_bound,
                        query: "FROM allEvents".parse()?,
                        order: Order::Asc,
                    })
                    .await?
                    .collect::<Vec<_>>()
                    .await;

                assert_eq!(result.len(), count);
                Ok(result)
            })
            .await?;
        }

        let (first, rest) = sim.machines_mut().split_at_mut(1);
        let first = &mut first[0];
        for machine in rest.iter_mut() {
            machine.send(Command::AddAddress(first.peer_id(), first.multiaddr()));
        }
        for machine in rest.iter_mut() {
            select_single(
                machine,
                Duration::from_secs(3),
                |ev| m!(ev, Event::Connected(peer) if *peer == first.peer_id() => ()),
            )
            .await;
        }
        for _ in 0..rest.len() {
            select_single(first, Duration::from_secs(3), |ev| matches!(ev, Event::Connected(_))).await;
        }

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

                        let result = api
                            .query(QueryRequest {
                                lower_bound: None,
                                upper_bound,
                                query: "FROM allEvents".parse()?,
                                order: Order::Asc,
                            })
                            .await?
                            .collect::<Vec<_>>()
                            .await;

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
