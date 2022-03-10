#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use actyx_sdk::{
        service::{EventService, Order, PublishEvent, PublishRequest, QueryRequest, QueryResponse},
        tags, Payload,
    };
    use anyhow::Context;
    use escargot::CargoBuild;
    use flate2::read::GzDecoder;
    use futures::{future::ready, StreamExt};
    use netsim_embed::{Ipv4Range, MachineId, Netsim};
    use std::{
        collections::{HashMap, HashSet},
        fs::{create_dir, File},
        path::{Path, PathBuf},
        thread::sleep,
        time::{Duration, Instant},
    };
    use swarm_cli::{Command, Event};
    use swarm_harness::{api::Api, setup_env, util::app_manifest, MachineExt};
    use tempdir::TempDir;
    use util::formats::os_arch::Arch;

    fn get_version(tmp: &Path, version: &str) -> anyhow::Result<PathBuf> {
        if version == "current" {
            Ok(CargoBuild::new()
                .manifest_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"))
                .current_release()
                .bin("actyx")
                .run()?
                .path()
                .to_owned())
        } else {
            let arch = match Arch::current() {
                Arch::x86_64 => "amd64",
                Arch::aarch64 => "arm64",
                Arch::arm => "arm",
                Arch::armv7 => "armhf",
                x => panic!("unsupported arch: {}", x),
            };
            let name = format!("actyx-{}-linux-{}", version, arch);
            let url = format!("https://axartifacts.blob.core.windows.net/releases/{}.tar.gz", name);
            let target = tmp.join(&name);
            let tgz = tmp.join(format!("{}.tgz", name));

            let mut file = File::create(&tgz).context("creating file")?;
            tracing::info!(url = %&url, tgz = %tgz.display(), "storing");
            let mut resp = reqwest::blocking::get(url).context("making request")?;
            let bytes = resp.copy_to(&mut file).context("storing to file")?;
            drop(file);
            tracing::info!(tgz = %tgz.display(), "written {} bytes", bytes);

            let file = File::open(&tgz).context("opening file")?;
            let zip = GzDecoder::new(file);
            let mut archive = tar::Archive::new(zip);
            create_dir(&target).context("creating version dir")?;
            archive.unpack(&target)?;

            Ok(target.join("actyx"))
        }
    }

    let versions = ["2.0.0", "2.3.0", "2.5.0", "2.8.2", "2.10.0", "2.11.0", "current"];

    setup_env().context("setting up env")?;
    let tmp_dir = TempDir::new("swarm-versions").context("creating temp_dir")?;
    let tmp = tmp_dir.path();
    create_dir(tmp.join("bin")).context("creating ./bin")?;

    match async_global_executor::block_on(async move {
        let mut sim = Netsim::<Command, Event>::new();

        let net = sim.spawn_network(Ipv4Range::random_local_subnet());
        tracing::warn!("using network {:?}", sim.network(net).range());

        for (idx, version) in versions.iter().copied().enumerate() {
            let mut cmd = async_process::Command::new(
                get_version(tmp.join("bin").as_path(), version).with_context(|| format!("get_version({})", version))?,
            );
            cmd.args([
                "--working-dir",
                tmp.join(idx.to_string()).display().to_string().as_str(),
                "--bind-api=0.0.0.0:4454",
            ]);
            let machine = sim.spawn_machine(cmd, None).await;
            sim.plug(machine, net, None).await;
            let m = sim.machine(machine);
            tracing::warn!(
                "{} started with address {} and peer id {}",
                machine,
                m.addr(),
                m.peer_id()
            );
        }

        let machines = sim.machines().iter().map(|m| m.id()).collect::<Vec<_>>();

        let started = Instant::now();
        for i in &machines {
            let ip = sim.machine(*i).addr();
            let ns = sim.machine(*i).namespace();
            loop {
                let req = std::thread::spawn(move || {
                    ns.enter().unwrap();
                    reqwest::blocking::get(format!("http://{}:4454/api/v2/node/id", ip))
                })
                .join()
                .unwrap();
                match req {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::info!("{} not yet ready: {}", i, e);
                        sleep(Duration::from_secs(1));
                    }
                }
                if started.elapsed() > Duration::from_secs(10) {
                    anyhow::bail!("timeout waiting for machines to come up");
                }
            }
        }

        let api = Api::with_port(&mut sim, app_manifest(), 4454).context("creating Api")?;
        let mut streams = HashMap::new();
        for i in &machines {
            let node_id = api.run(*i, |api| async move { Ok(api.node_id().await) }).await?;
            streams.insert(node_id.stream(0.into()), *i);
        }
        let all = machines.iter().copied().collect::<HashSet<_>>();

        // step 1: wait until events propagate, i.e. all are connected
        for i in &machines {
            let r = api
                .run(*i, |api| async move {
                    api.publish(PublishRequest {
                        data: vec![PublishEvent {
                            tags: tags!("versions"),
                            payload: Payload::from_json_str("1").map_err(|s| anyhow::anyhow!("{}", s))?,
                        }],
                    })
                    .await
                })
                .await?;
            tracing::info!("{} published: {:?}", i, r);
        }

        let started = Instant::now();
        for i in &machines {
            loop {
                let v = api
                    .run(*i, |api| async move {
                        api.query(QueryRequest {
                            lower_bound: None,
                            upper_bound: None,
                            query: "FROM 'versions'".parse()?,
                            order: Order::Asc,
                        })
                        .await
                    })
                    .await?
                    .filter_map(|r| {
                        tracing::info!("{} query response {:?}", i, r);
                        match r {
                            QueryResponse::Event(e) => ready(streams.get(&e.stream)),
                            _ => ready(None),
                        }
                    })
                    .collect::<HashSet<MachineId>>()
                    .await;
                if v == all {
                    break;
                }
                if started.elapsed() > Duration::from_secs(20) {
                    anyhow::bail!("timeout waiting for event propagation");
                }
                sleep(Duration::from_secs(1));
            }
        }

        // it may be that the above was fulfilled by indirect event delivery, but now every
        // node must see every other; just wait a second to get them connected
        sleep(Duration::from_secs(1));

        // step 2: check that fast_path is working between all of them
        for i in &machines {
            let r = api
                .run(*i, |api| async move {
                    api.publish(PublishRequest {
                        data: vec![PublishEvent {
                            tags: tags!("version2"),
                            payload: Payload::from_json_str("1").map_err(|s| anyhow::anyhow!("{}", s))?,
                        }],
                    })
                    .await
                })
                .await?;
            tracing::info!("{} published: {:?}", i, r);
        }

        let started = Instant::now();
        for i in &machines {
            loop {
                let v = api
                    .run(*i, |api| async move {
                        api.query(QueryRequest {
                            lower_bound: None,
                            upper_bound: None,
                            query: "FROM 'version2'".parse()?,
                            order: Order::Asc,
                        })
                        .await
                    })
                    .await?
                    .filter_map(|r| {
                        tracing::info!("{} query response {:?}", i, r);
                        match r {
                            QueryResponse::Event(e) => ready(streams.get(&e.stream)),
                            _ => ready(None),
                        }
                    })
                    .collect::<HashSet<MachineId>>()
                    .await;
                if v == all {
                    break;
                }
                if started.elapsed() > Duration::from_secs(1) {
                    anyhow::bail!("timeout waiting for event propagation");
                }
                sleep(Duration::from_millis(100));
            }
        }

        Ok(())
    }) {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!("persisting tmp dir at {}", tmp_dir.into_path().display());
            Err(e)
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    panic!("this test can only be run on Linux");
}
