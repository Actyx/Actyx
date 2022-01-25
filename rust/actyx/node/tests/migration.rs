use actyx_sdk::service::EventService;
use actyx_sdk::{app_id, AppManifest, Offset, StreamId};
use escargot::{format::Message, CargoBuild};
use std::net::{IpAddr, Ipv4Addr, TcpListener};
use std::path::{self, Path};
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Once;
use std::time::Duration;
use std::{fs, io};
use tempfile::tempdir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

const FEATURES: &str = "migration-v1";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}
impl Version {
    pub fn new(s: &str) -> Self {
        let s = if let Some(pos) = s.find('-') {
            s.split_at(pos).0
        } else {
            s
        };
        let mut parts = s.split('.');
        Self {
            major: parts.next().unwrap().parse().unwrap(),
            minor: parts.next().unwrap().parse().unwrap(),
            patch: parts.next().unwrap().parse().unwrap(),
        }
    }
}

fn setup() {
    // make sure actyx binary is built and available
    // (so you don't have to spend scratching your head about the code that is being run ..)
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        util::setup_logger();
        // build needed binaries for quicker execution
        eprintln!("building actyx");
        for msg in CargoBuild::new()
            .manifest_path("../Cargo.toml")
            .bin("actyx")
            .features(FEATURES)
            .exec()
            .unwrap()
        {
            let msg = msg.unwrap();
            let msg = msg.decode().unwrap();
            match msg {
                Message::BuildFinished(x) => eprintln!("{:?}", x),
                Message::CompilerArtifact(a) => {
                    if !a.fresh {
                        eprintln!("{:?}", a.package_id)
                    }
                }
                Message::CompilerMessage(s) => {
                    if let Some(msg) = s.message.rendered {
                        eprintln!("{}", msg)
                    }
                }
                Message::BuildScriptExecuted(_) => {}
                Message::Unknown => {}
            }
        }
    });
}

#[tokio::test]
#[cfg(feature = "migration-v1")]
async fn migration() -> anyhow::Result<()> {
    // Run these tests sequentially
    migration_dir().await?;
    migration_v1_find_old_working_dir().await?;
    Ok(())
}

#[allow(unused)]
async fn migration_dir() -> anyhow::Result<()> {
    setup();
    for entry in fs::read_dir("tests/migration-test-data")? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            println!("testing dir {}", p.display());
            let tmp = tempdir()?;
            copy_dir_recursive(&p, tmp.path())?;

            match p.file_name().unwrap().to_str().unwrap() {
                "1.1.5-without-events" => assert_v2_from_v1_files(tmp.path(), std::iter::empty(), 1).await,
                "1.0.0" => {
                    assert_v2_from_v1_files(
                        tmp.path(),
                        std::iter::once((
                            StreamId::from_str("Zg/1L3Tm5xWNV94nFsjaIO8s3kW6Sj1y4fzQR5zcVeo-0")?,
                            4.into(),
                        )),
                        0,
                    )
                    .await
                }
                x if x.starts_with("1.") => {
                    assert_v2_from_v1_files(
                        tmp.path(),
                        std::iter::once((
                            StreamId::from_str("pEZIcZtKHtHuV.JbKrldCcUnvIY6Y2f2U4L3oofMVL6-0")?,
                            10.into(),
                        )),
                        1,
                    )
                    .await
                }
                n => {
                    let version = Version::new(n);
                    if version < Version::new("2.9.0") {
                        assert_v2_migration(
                            tmp.path(),
                            std::iter::once((
                                StreamId::from_str("pEZIcZtKHtHuV.JbKrldCcUnvIY6Y2f2U4L3oofMVL6-0")?,
                                9.into(),
                            )),
                            2,
                        )
                        .await
                    } else {
                        try_run(
                            tmp.path(),
                            std::iter::once((
                                StreamId::from_str("pEZIcZtKHtHuV.JbKrldCcUnvIY6Y2f2U4L3oofMVL6-0")?,
                                9.into(),
                            )),
                        )
                        .await
                        .map(|_| {})
                    }
                }
            }
            .map_err(|e| {
                println!(
                    "Error during testing, persisted temporary dir {}",
                    tmp.into_path().display()
                );
                e
            })?;
        }
    }
    Ok(())
}

#[allow(unused)]
async fn migration_v1_find_old_working_dir() -> anyhow::Result<()> {
    setup();
    // actyxos: ActyxOS on Docker v1
    // actyxos-data: Default for Actyx on Linux
    for old_dir_name in ["actyxos-data", "actyxos"].iter() {
        let tmp = tempdir()?;
        copy_dir_recursive("tests/migration-test-data/1.1.5", tmp.path().join(old_dir_name))?;
        let v2_working_dir = tmp.path().join("v2_working_dir");
        fs::create_dir(&v2_working_dir)?;

        assert_v2_from_v1_files(
            v2_working_dir,
            std::iter::once((
                StreamId::from_str("pEZIcZtKHtHuV.JbKrldCcUnvIY6Y2f2U4L3oofMVL6-0")?,
                10.into(),
            )),
            1,
        )
        .await
        .map_err(|e| {
            println!(
                "Error during testing, persisted temporary dir {}",
                tmp.into_path().display()
            );
            e
        })?;
    }
    Ok(())
}

async fn assert_v2_from_v1_files(
    working_dir: impl AsRef<Path>,
    expected_offsets: impl Iterator<Item = (StreamId, Offset)>,
    initial_db_version: u32,
) -> anyhow::Result<()> {
    let stderr = try_run(&working_dir, expected_offsets).await?;

    let backup_file = fs::read_dir(working_dir.as_ref())?
        .into_iter()
        .filter_map(|x| x.ok())
        .find(|x| {
            x.file_name()
                .to_string_lossy()
                .to_string()
                .ends_with("v1_data_files.tar.gz")
        })
        .ok_or_else(|| anyhow::anyhow!("Couldn't find backup file"))?;

    for (actual, expected) in stderr
        .into_iter()
        .filter(|x| {
            if initial_db_version < 2 && x.contains("ipfs_sqlite_block_store") {
                // ipfs_sqlite_block_store prints out some migration logs when coming from v0 / v1
                false
            } else {
                !x.contains("wal_checkpoint")
            }
        })
        .take(6)
        .zip(
            vec![
                format!("using data directory `{}`", working_dir.as_ref().display()),
                format!(
                    "Migrating data from an earlier version ({} to 2) ..",
                    initial_db_version
                ),
                format!("Created backup of v1 files at {}", backup_file.path().display()),
                "Migration succeeded.".to_string(),
                "Migrating data from an earlier version (2 to 3) ..".to_string(),
                "Migration succeeded.".to_string(),
            ]
            .into_iter(),
        )
    {
        anyhow::ensure!(actual.ends_with(&*expected), "'{}' != '{}'", actual, expected);
    }
    Ok(())
}

async fn assert_v2_migration(
    working_dir: impl AsRef<Path>,
    expected_offsets: impl Iterator<Item = (StreamId, Offset)>,
    initial_db_version: u32,
) -> anyhow::Result<()> {
    let stderr = try_run(&working_dir, expected_offsets).await?;
    for (actual, expected) in stderr
        .into_iter()
        .filter(|x| {
            if initial_db_version == 0 && x.contains("ipfs_sqlite_block_store") {
                // ipfs_sqlite_block_store prints out some migration logs when coming from v0
                false
            } else {
                !x.contains("wal_checkpoint")
            }
        })
        .take(3)
        .zip(
            vec![
                format!("using data directory `{}`", working_dir.as_ref().display()),
                "Migrating data from an earlier version (2 to 3) ..".to_string(),
                "Migration succeeded.".to_string(),
            ]
            .into_iter(),
        )
    {
        anyhow::ensure!(actual.ends_with(&*expected), "'{}' != '{}'", actual, expected);
    }
    Ok(())
}

async fn try_run(
    working_dir: impl AsRef<Path>,
    expected_offsets: impl Iterator<Item = (StreamId, Offset)>,
) -> anyhow::Result<Vec<String>> {
    let ports = (0..3)
        .map(|_| TcpListener::bind((IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0)).map_err(Into::into))
        .collect::<anyhow::Result<Vec<_>>>()?
        // keeping the listeners alive avoids getting the same port number twice
        .into_iter()
        .map(|l| Ok(l.local_addr()?.port()))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let mut child = Command::new(target_dir().join("actyx"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(&[
            "--bind-admin",
            &*ports[0].to_string(),
            "--bind-api",
            &*ports[1].to_string(),
            "--bind-swarm",
            &*ports[2].to_string(),
            "--working-dir",
            &*working_dir.as_ref().to_string_lossy(),
        ])
        .env(
            "RUST_LOG",
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        )
        .kill_on_drop(true)
        .spawn()?;

    let output = child.stderr.take().unwrap();
    let mut reader = BufReader::new(output).lines();
    let mut stderr = vec![];

    let mut started = false;
    while !started {
        let l = timeout(Duration::from_millis(5000), reader.next_line())
            .await??
            .ok_or_else(|| anyhow::anyhow!("EOF"))?;
        println!("stderr: {}", l);
        started = l.contains("NODE_STARTED_BY_HOST");
        stderr.push(l);
    }
    // The `actyx` process may get blocked because we don't continuie to
    // read its stdout/stderr. This shouldn't be a problem for those short-lived
    // tests, but might be wity extremely verbose logging.
    let client = actyx_sdk::HttpClient::new(
        format!("http://localhost:{}", ports[1]).parse().unwrap(),
        AppManifest::new(
            app_id!("com.example.trial-mode"),
            "display name".into(),
            "0.1.0".into(),
            None,
        ),
    )
    .await?;
    let offset_map = client.offsets().await?;
    // Check node key and event migration
    for (stream, offset) in expected_offsets {
        assert_eq!(offset_map.present.get(stream), Some(offset));
    }
    Ok(stderr)
}

fn copy_dir_recursive(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_recursive(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn target_dir() -> path::PathBuf {
    std::env::current_exe()
        .ok()
        .map(|mut path| {
            path.pop();
            if path.ends_with("deps") {
                path.pop();
            }
            path
        })
        .unwrap()
}
