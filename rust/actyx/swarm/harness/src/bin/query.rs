#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use anyhow::Context;
    use async_std::{
        future::timeout,
        task::{block_on, sleep},
    };
    use ax_sdk::types::{
        service::{PublishEvent, QueryResponse},
        tags, AppManifest, OffsetMap, Payload,
    };
    use futures::{future, StreamExt};
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        time::Duration,
    };
    use structopt::StructOpt;
    use swarm_cli::Event;
    use swarm_harness::{api::Api, fully_meshed, HarnessOpts};

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
        let api = Api::new(&mut sim, AppManifest::default())?;

        for (idx, machine) in sim.machines().iter().enumerate() {
            tracing::error!("{}", idx);
            api.run(machine.id(), |api| async move {
                api.execute(|ax| {
                    block_on(ax.publish().events((0..N).map(|i| PublishEvent {
                        tags: tags!("a", "b"),
                        payload: Payload::from_json_str(&format!("{}", i)).unwrap(),
                    })))
                })
                .await??;

                let upper_bound = api.offsets().await?.present;
                let count = (&upper_bound - &OffsetMap::default()) as usize;
                assert!(count >= N);

                let result = api
                    .execute(|ax| block_on(ax.query("FROM allEvents")))
                    .await??
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
                            api.execute(move |ax| block_on(ax.query("FROM allEvents").with_upper_bound(upper_bound)))
                                .await??
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
