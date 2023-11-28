#[cfg(target_os = "linux")]
mod versions {
    use anyhow::Context;
    use async_std::task::block_on;
    use ax_core::util::version::ARCH;
    use ax_sdk::{
        service::{EventMeta, EventResponse, QueryResponse},
        StreamId, TagSet,
    };
    use escargot::CargoBuild;
    use flate2::read::GzDecoder;
    use futures::{future::ready, StreamExt};
    use netsim_embed::{Ipv4Range, MachineId, Netsim};
    use std::{
        collections::{HashMap, HashSet},
        fs::{create_dir, File},
        path::{Path, PathBuf},
        time::{Duration, Instant},
    };
    use swarm_cli::{Command, Event};
    use swarm_harness::{api::Api, util::app_manifest, MachineExt};

    const VERSIONS: [&str; 7] = ["2.0.0", "2.3.0", "2.5.0", "2.8.2", "2.10.0", "2.11.0", "current"];

    enum VersionPathBuf {
        Ax(PathBuf),    // new `ax run`
        Actyx(PathBuf), // old `actyx` bin
    }

    impl VersionPathBuf {
        fn to_command(&self) -> async_process::Command {
            match self {
                VersionPathBuf::Ax(path_buf) => {
                    let mut cmd = async_process::Command::new(path_buf);
                    cmd.arg("run");
                    cmd
                }
                VersionPathBuf::Actyx(path_buf) => async_process::Command::new(path_buf),
            }
        }
        fn get_version(tmp: &Path, version: &str) -> anyhow::Result<VersionPathBuf> {
            if version == "current" {
                Ok(VersionPathBuf::Ax(
                    CargoBuild::new()
                        .manifest_path(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"))
                        .current_release()
                        .bin("ax")
                        .run()?
                        .path()
                        .to_owned(),
                ))
            } else {
                let name = format!("actyx-{}-linux-{}", version, ARCH);
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

                Ok(VersionPathBuf::Actyx(target.join("actyx")))
            }
        }
    }

    pub async fn spawn_network(sim: &mut Netsim<Command, Event>, tmp: &Path) -> anyhow::Result<()> {
        let net = sim.spawn_network(Ipv4Range::random_local_subnet());
        tracing::warn!("using network {:?}", sim.network(net).range());

        for (idx, version) in VERSIONS.iter().copied().enumerate() {
            let mut cmd = VersionPathBuf::get_version(tmp, version)?.to_command();
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
        Ok(())
    }

    async fn ensure_all_machines_are_up(
        sim: &mut Netsim<Command, Event>,
        machine_ids: &Vec<MachineId>,
    ) -> anyhow::Result<()> {
        let start = Instant::now();
        for i in machine_ids {
            let ip = sim.machine(*i).addr();
            let ns = sim.machine(*i).namespace();
            loop {
                // Spawning a "proper" thread is required because of `ns.enter()`
                // which places the thread in a given namespace, tokio is not helpful
                // here because it will keep the thread around - inside said namespace.
                // Hence, disposing of the thread is how we "leave" the namespace
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
                        async_std::task::sleep(Duration::from_secs(1)).await;
                    }
                }
                if start.elapsed() > Duration::from_secs(10) {
                    anyhow::bail!("timeout waiting for machines to come up");
                }
            }
        }
        Ok(())
    }

    /// This function:
    /// 1. publishes a single message as each machine
    /// 2. queries for said messages in each machine
    ///
    /// This process ensures that all machines have connected and
    /// depending on timing constraints, that the fast path is being use.
    /// This is controlled by the `timeout` and `interval` parameters.
    /// - The `timeout` parameter controls for how long we're willing to wait until we
    /// receive messages from all machines.
    /// - The `interval` parameter controls how much time we wait between each query.
    async fn publish_and_query(
        api: &Api,
        machine_ids: &Vec<MachineId>,
        streams: &HashMap<StreamId, MachineId>,
        tag: &'static str,
        timeout: Duration,
        interval: Duration,
    ) -> anyhow::Result<()> {
        let all = machine_ids.iter().copied().collect::<HashSet<_>>();

        for i in machine_ids {
            let r = api
                .run(*i, |api| async move {
                    api.execute(|ax| {
                        let tags = TagSet::from_iter([tag.parse().expect("A valid tag")]);
                        let event = serde_json::json!("1");
                        block_on(ax.publish().event(tags, &event).unwrap())
                    })
                    .await
                    .unwrap()
                })
                .await?;
            tracing::info!("{} published: {:?}", i, r);
        }

        for i in machine_ids {
            let _: anyhow::Result<()> = async_std::future::timeout(timeout, async {
                loop {
                    let alive_machines = api
                        .run(*i, |api| async move {
                            api.execute(move |ax| block_on(ax.query(format!("FROM '{}'", tag))))
                                .await
                                .unwrap()
                        })
                        .await?
                        .filter_map(|r| {
                            tracing::info!("{} query response {:?}", i, r);
                            match r {
                                QueryResponse::Event(EventResponse {
                                    meta: EventMeta::Event { key, .. },
                                    ..
                                }) => ready(streams.get(&key.stream)),
                                _ => ready(None),
                            }
                        })
                        .collect::<HashSet<MachineId>>()
                        .await;
                    if alive_machines == all {
                        break;
                    }
                    async_std::task::sleep(interval).await;
                }
                Ok(())
            })
            .await
            .context("timeout waiting for event propagation")?;
        }

        Ok(())
    }

    pub async fn run(tmp: &Path) -> anyhow::Result<()> {
        let mut sim = Netsim::<Command, Event>::new();
        spawn_network(&mut sim, tmp).await?;
        let machines = sim.machines().iter().map(|m| m.id()).collect::<Vec<_>>();
        ensure_all_machines_are_up(&mut sim, &machines).await?;

        let api = Api::with_port(&mut sim, app_manifest(), 4454).context("creating Api")?;
        let mut streams = HashMap::new();
        for i in &machines {
            let node_id = api.run(*i, |api| async move { Ok(api.node_id().await) }).await?;
            streams.insert(node_id.stream(0.into()), *i);
        }

        // Step 1: wait until events propagate, i.e. all are connected
        // We're using a total timeout of 20 seconds and an interval of 1 second between queries
        // These values date back to before the refactor
        publish_and_query(
            &api,
            &machines,
            &streams,
            "versions",
            Duration::from_secs(20),
            Duration::from_secs(1),
        )
        .await?;

        // it may be that the above was fulfilled by indirect event delivery, but now every
        // node must see every other; just wait a second to get them connected
        async_std::task::sleep(Duration::from_secs(1)).await;

        // step 2: check that fast_path is working between all of them
        // We're using a total timeout of 1 seconds and an interval of 100 milliseconds between queries
        // These values date back to before the refactor
        publish_and_query(
            &api,
            &machines,
            &streams,
            "version2",
            Duration::from_secs(1),
            Duration::from_millis(100),
        )
        .await?;

        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    use anyhow::Context;
    use std::fs::create_dir;
    use swarm_harness::setup_env;
    use tempdir::TempDir;

    setup_env().context("setting up env")?;
    let tmp_dir = TempDir::new("swarm-versions").context("creating temp_dir")?;
    let tmp = tmp_dir.path();

    create_dir(tmp.join("bin")).context("creating ./bin")?;

    match async_global_executor::block_on(versions::run(tmp)) {
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
