#![cfg(target_os = "linux")]
use anyhow::{anyhow, bail, ensure};
use ax_core::{
    cmd::ActyxCliResult,
    util::{
        formats::{ActyxOSCode, NodesInspectResponse},
        version::Version,
    },
};
use ax_sdk::service::OffsetsResponse;
use escargot::{format::Message, CargoBuild};
use flate2::read::GzDecoder;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::{
    env,
    ffi::OsStr,
    fmt::Write,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tar::Archive;
use tempfile::tempdir;

trait Opts: Sized {
    type Out;
    fn v(self, msg: &str) -> anyhow::Result<Self::Out>;
}
impl<T> Opts for Option<T> {
    type Out = T;
    fn v(self, msg: &str) -> anyhow::Result<T> {
        self.ok_or_else(|| anyhow!("{}: no value", msg))
    }
}

#[derive(Clone, Default)]
struct Log(Arc<Mutex<String>>);
impl Write for Log {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.lock().write_str(s)
    }
}
impl std::fmt::Display for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.lock())
    }
}

struct Binaries {
    cli: Vec<(Version, PathBuf)>,
    actyx: Vec<(Version, PathBuf)>,
    ax: Vec<(Version, PathBuf)>,
}

const VERSIONS: &str = "../../../versions";
const ROOT_URL: &str = "https://axartifacts.blob.core.windows.net/releases";

fn setup() -> &'static Binaries {
    static INIT: OnceCell<Binaries> = OnceCell::new();
    INIT.get_or_init(|| {
        // build needed binaries for quicker execution
        for bin in &["ax"] {
            eprintln!("building {}", bin);
            for msg in CargoBuild::new()
                .manifest_path("../Cargo.toml")
                .bin(*bin)
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
        }

        let storage_dir = PathBuf::from(
            env::var_os("ACTYX_BINARIES")
                .or_else(|| {
                    env::var_os("HOME").map(|mut home| {
                        home.push("/actyx_binaries");
                        home
                    })
                })
                .unwrap_or_else(|| ".".into()),
        );
        std::fs::create_dir_all(&storage_dir)
            .unwrap_or_else(|e| panic!("cannot create {}: {}", storage_dir.display(), e));

        let mut actyx = vec![];
        let mut cli = vec![];
        let mut ax = vec![];

        // the newest versions may not yet be uploaded, especially when validating the release PR
        let mut may_skip_actyx = true;
        let mut may_skip_cli = true;
        let mut may_skip_ax = true;
        for line in BufReader::new(File::open(VERSIONS).unwrap_or_else(|e| panic!("cannot open {}: {}", VERSIONS, e)))
            .lines()
            .map(|line| line.unwrap())
        {
            if line.starts_with("actyx-") {
                let end = line
                    .find(' ')
                    .unwrap_or_else(|| panic!("malformatted `actyx-` line in versions"));
                let version =
                    Version::from_str(&line[6..end]).unwrap_or_else(|_e| panic!("malformed version {}", line));
                if version == Version::new(1, 1, 5) {
                    continue;
                }
                let path = download("actyx", "actyx", version, &storage_dir, &mut may_skip_actyx);
                if let Some(path) = path {
                    actyx.push((version, path))
                }
            }
            if line.starts_with("cli-") {
                let end = line
                    .find(' ')
                    .unwrap_or_else(|| panic!("malformatted `cli-` line in versions"));
                let version =
                    Version::from_str(&line[4..end]).unwrap_or_else(|_e| panic!("malformed version {}", line));
                if version == Version::new(1, 1, 5) {
                    continue;
                }
                let path = download("actyx-cli", "ax", version, &storage_dir, &mut may_skip_cli);
                if let Some(path) = path {
                    cli.push((version, path))
                }
            }
            if line.starts_with("ax-") {
                let end = line
                    .find(' ')
                    .unwrap_or_else(|| panic!("malformatted `ax-` line in versions"));
                let version =
                    Version::from_str(&line[3..end]).unwrap_or_else(|_e| panic!("malformed version {}", line));
                let path = download("ax", "ax", version, &storage_dir, &mut may_skip_ax);
                if let Some(path) = path {
                    ax.push((version, path))
                }
            }
        }

        Binaries { actyx, cli, ax }
    })
}

fn download(package: &str, bin: &str, version: Version, dst_dir: &Path, may_skip: &mut bool) -> Option<PathBuf> {
    // This is a bit of an abuse but we're currently running this only on 64-bit Linux
    // hence taking the liberty to "force" this here. For a reference, the previous
    // mapping was:
    // - x86_64 => amd64
    // - aarch64 => arm64
    // - arm => arm (stopped being supported during the move to ax)
    // - armv7 => armhf
    let name = format!("{}-{}-linux-amd64", package, version);
    let url = format!("{}/{}.tar.gz", ROOT_URL, name);
    let target = dst_dir.join(&name);

    match target.metadata() {
        Ok(meta) if meta.is_file() && meta.len() > 0 => {
            println!("assuming {} version {} is already there", bin, version);
            return Some(target);
        }
        _ => println!("storing {} from {} into {}", bin, url, target.display()),
    }

    let resp = reqwest::blocking::get(&url).unwrap_or_else(|e| panic!("making request to {}: {}", url, e));
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        panic!("did not find {}", url);
    }
    let gzip = GzDecoder::new(resp);
    let mut archive = Archive::new(gzip);
    let entries = match archive.entries() {
        Ok(e) => e,
        Err(_) if *may_skip => {
            *may_skip = false;
            return None;
        }
        x => x.unwrap(),
    };
    for entry in entries {
        let mut entry = match entry {
            Ok(e) => e,
            Err(_) if *may_skip => {
                *may_skip = false;
                return None;
            }
            x => x.unwrap(),
        };
        let path = entry.path().unwrap_or_else(|e| panic!("getting path: {}", e));
        if entry.header().entry_type().is_file() && path.as_ref() == Path::new(bin) {
            entry
                .unpack(&target)
                .unwrap_or_else(|e| panic!("unpacking {}: {}", version, e));
            return Some(target);
        } else {
            println!("skipping {:?} {}", entry.header().entry_type(), path.display());
        }
    }
    panic!("archive at {} did not contain {}", url, bin);
}

fn run(bin: &str) -> anyhow::Result<Command> {
    Ok(CargoBuild::new()
        .manifest_path("../Cargo.toml")
        .bin(bin)
        .run()?
        .command())
}

fn with_api(
    cmd: &mut Command,
    use_stdout: bool,
    mut log: impl Write + Clone + Send + 'static,
    f: impl FnOnce(u16, &Path) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    ax_core::util::setup_logger();
    setup();

    let workdir = tempdir()?;

    let _ = writeln!(log, "running test in {}", std::env::current_dir()?.display());
    let mut process = cmd
        .current_dir(workdir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(["--bind-api=0", "--bind-admin=0", "--bind-swarm=0"])
        .env("RUST_LOG", "debug")
        .spawn()?;
    let logging: Box<dyn Read + Send + 'static> = if use_stdout {
        Box::new(process.stdout.take().unwrap())
    } else {
        Box::new(process.stderr.take().unwrap())
    };

    let identity = workdir.path().join("identity");
    let mut args = ["users", "keygen", "-jo"].iter().map(OsStr::new).collect::<Vec<_>>();
    args.push(identity.as_os_str());
    let keygen = run("ax")?.args(args).output()?;
    ensure!(
        keygen.status.success(),
        "out: {}err: {}",
        String::from_utf8_lossy(&keygen.stdout),
        String::from_utf8_lossy(&keygen.stderr)
    );
    let _ = writeln!(log, "identity: {}", String::from_utf8(keygen.stdout)?);

    // ensure that the test ends at some point
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let mut rx = Some((rx, process));

    let mut lines = BufReader::new(logging).lines();
    let mut api = 0u16;
    for line in &mut lines {
        if let Some((rx, mut process)) = rx.take() {
            // unfortunately escargot doesn’t inform us when building is finished,
            // so we start the Actyx timeout upon seeing the first line of output
            thread::spawn(move || {
                let _ = rx.recv_timeout(Duration::from_secs(10));
                eprintln!("killing Actyx");
                let _ = process.kill();
            });
        }

        let line = line?;
        let _ = writeln!(log, "line: {}", line);
        if line.contains("ADMIN_API_BOUND") {
            const HOST: &str = "127.0.0.1/tcp/";
            if let Some(idx) = line.find(HOST) {
                let idx = idx + HOST.len();
                let upper = line[idx..]
                    .find(|c: char| !c.is_ascii_digit())
                    .map(|i| idx + i)
                    .unwrap_or_else(|| line.len());
                api = line[idx..upper].parse()?;
                break;
            }
        } else if line.contains("NODE_STARTED_BY_HOST") {
            bail!("no ADMIN_API_BOUND logged");
        }
    }
    if api == 0 {
        bail!("startup timed out");
    }
    let _ = writeln!(log, "found port {}", api);
    let mut log2 = log.clone();
    let handle = thread::spawn(move || {
        for line in lines.flatten() {
            let _ = writeln!(log2, "line: {}", line);
        }
    });

    let started = Instant::now();
    loop {
        let err = match get_offsets(api, identity.as_ref()) {
            Ok(ActyxCliResult::OK { .. }) => break,
            Ok(ActyxCliResult::ERROR(e)) if e.code() == ActyxOSCode::ERR_UNSUPPORTED => break,
            Ok(ActyxCliResult::ERROR(e)) => anyhow::Error::from(e),
            Err(e) => e,
        };
        if started.elapsed() > Duration::from_secs(5) {
            return Err(err);
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // run the test
    let result = f(api, identity.as_ref());

    let _ = writeln!(log, "killing process");
    let _ = tx.send(());
    let _ = handle.join();
    result
}

fn get_offsets(api: u16, identity: &Path) -> anyhow::Result<ActyxCliResult<OffsetsResponse>> {
    let out = run("ax")?
        .args([
            o("events"),
            o("offsets"),
            o("-ji"),
            identity.as_os_str(),
            o(&format!("127.0.0.1:{}", api)),
        ])
        .env("RUST_LOG", "debug")
        .output()?;
    println!(
        "prep out:\n{}\nerr:\n{}\n---",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let v = serde_json::from_slice::<ActyxCliResult<OffsetsResponse>>(&out.stdout)?;
    Ok(v)
}

fn o(s: &str) -> &OsStr {
    OsStr::new(s)
}

#[test]
fn all_ax() -> anyhow::Result<()> {
    let binaries = setup();
    let log = Log::default();
    let result = with_api(
        run("ax").unwrap().args(["run"]),
        false,
        log.clone(),
        |port, identity| {
            for (version, ax) in binaries.cli.iter().chain(binaries.ax.iter()) {
                println!("testing {}", version);
                if *version >= Version::new(2, 1, 0) {
                    let out = Command::new(ax)
                        .args([
                            o("events"),
                            o("offsets"),
                            o("-ji"),
                            identity.as_os_str(),
                            o(&format!("127.0.0.1:{}", port)),
                        ])
                        .env("RUST_LOG", "info")
                        .output()?;
                    println!(
                        "offsets out:\n{}\nerr:\n{}---\n",
                        String::from_utf8_lossy(&out.stdout),
                        String::from_utf8_lossy(&out.stderr)
                    );
                    ensure!(out.status.success());
                }
                let out = Command::new(ax)
                    .args([
                        o("nodes"),
                        o("inspect"),
                        o("-ji"),
                        identity.as_os_str(),
                        o(&format!("127.0.0.1:{}", port)),
                    ])
                    .env("RUST_LOG", "debug")
                    .output()?;
                println!(
                    "out:\n{}\nerr:\n{}---\n",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                ensure!(out.status.success());
                let inspect = serde_json::from_slice::<ActyxCliResult<NodesInspectResponse>>(&out.stdout)?;
                let ActyxCliResult::OK { result, .. } = inspect else {
                    bail!("cli error: {:?}", inspect)
                };
                ensure!(result.admin_addrs.contains(&format!("/ip4/127.0.0.1/tcp/{}", port)));
            }
            Ok(())
        },
    );
    if result.is_err() {
        println!("{}", log);
    }
    result
}

#[test]
fn all_actyx() -> anyhow::Result<()> {
    let binaries = setup();
    let ax = run("ax")?;
    for (version, actyx) in binaries.actyx.iter().chain(binaries.ax.iter()) {
        let log = Log::default();
        let use_stdout_before = Version::new(2, 1, 0);
        let result = with_api(
            &mut Command::new(actyx),
            *version < use_stdout_before,
            log.clone(),
            |port, identity| {
                println!("testing version {}", version);
                let out = Command::new(ax.get_program())
                    .args([
                        o("nodes"),
                        o("inspect"),
                        o("-ji"),
                        identity.as_os_str(),
                        o(&format!("127.0.0.1:{}", port)),
                    ])
                    .env("RUST_LOG", "debug")
                    .output()?;
                println!(
                    "out:\n{}\nerr:\n{}---\n",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                ensure!(out.status.success());
                let inspect = serde_json::from_slice::<ActyxCliResult<NodesInspectResponse>>(&out.stdout)?;
                let ActyxCliResult::OK { result, .. } = inspect else {
                    bail!("cli error: {:?}", inspect)
                };
                ensure!(result.admin_addrs.contains(&format!("/ip4/127.0.0.1/tcp/{}", port)));
                Ok(())
            },
        );
        if result.is_err() {
            println!("{}", log);
            return result;
        }
    }
    Ok(())
}
