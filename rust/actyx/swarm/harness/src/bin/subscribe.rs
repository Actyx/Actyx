#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        time::Duration,
    };

    use actyx_sdk::{
        app_id,
        service::{EventService, PublishEvent, PublishRequest},
        tags, AppManifest, Payload, Timestamp,
    };
    use anyhow::Context;
    use async_std::future::timeout;
    use structopt::StructOpt;
    use swarm_cli::{Command, Event, TimedEvent};
    use swarm_harness::{api::Api, fully_meshed, m, HarnessOpts};

    fn event(n: usize) -> PublishRequest {
        PublishRequest {
            data: vec![PublishEvent {
                tags: tags!("a"),
                payload: Payload::from_json_str(&*format!("{}", n)).unwrap(),
            }],
        }
    }

    #[allow(clippy::just_underscores_and_digits)]
    fn percentiles<T: Ord>(mut v: Vec<T>) -> (T, T, T) {
        let max = v.len() - 1;
        v.sort();
        let _50 = v.remove(max / 2);
        let _95 = v.remove(max * 95 / 100);
        let _99 = v.remove(max * 99 / 100);
        (_50, _95, _99)
    }

    let app_manifest = AppManifest {
        app_id: app_id!("com.example.query"),
        display_name: "Query test".into(),
        version: "0.1.0".into(),
        signature: None,
    };

    const REPETITIONS: usize = 200;

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

        fully_meshed(&mut sim, Duration::from_secs(60)).await?;

        for machine in sim.machines_mut() {
            machine.send(Command::SubscribeQuery("FROM 'a'".parse()?));
        }

        for machine in sim.machines_mut() {
            for ev in machine.drain() {
                tracing::info!("{} got event {}", machine.id(), ev);
            }
        }

        let mut measurements = Vec::new();
        let ids = sim.machines().iter().map(|x| x.id()).collect::<Vec<_>>();
        for (n, id) in ids.iter().cycle().take(REPETITIONS).copied().enumerate() {
            let start = api
                .run(id, |api| async move {
                    let now = Timestamp::now();
                    api.publish(event(n)).await?;
                    Ok(now)
                })
                .await?;
            for machine in sim.machines_mut() {
                let (received, published) = timeout(
                    Duration::from_secs(5),
                    machine.select(|ev| {
                        m!(ev, TimedEvent { timestamp, event: Event::Result((_, key, payload)) } => {
                            assert_eq!(payload.json_string(), format!("{}", n));
                            (*timestamp, key.time())
                        })
                    }),
                )
                .await
                .with_context(|| format!("timeout waiting for message {} from {} to {}", n, id, machine.id()))?
                .unwrap();
                measurements.push((start, published, received));
            }
        }

        let (p50, p95, p99) = percentiles(measurements.iter().map(|x| x.2 - x.1).collect());
        let (q50, q95, q99) = percentiles(measurements.iter().map(|x| x.1 - x.0).collect());

        tracing::info!("transmission percentiles {}µs /{}µs /{}µs", p50, p95, p99);
        tracing::info!("publishing percentiles {}µs /{}µs /{}µs", q50, q95, q99);

        assert!(p99 < 1_000_000);

        Ok(())
    })
}

#[cfg(not(target_os = "linux"))]
fn main() {}
