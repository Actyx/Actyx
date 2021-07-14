use anyhow::{anyhow, bail, ensure};
use escargot::CargoBuild;
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::{
    fmt::Write,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    sync::{mpsc::channel, Arc},
    thread::spawn,
    time::Duration,
};
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

fn run(bin: &str) -> anyhow::Result<Command> {
    Ok(CargoBuild::new()
        .manifest_path("../Cargo.toml")
        .bin(bin)
        .run()?
        .command())
}

fn with_api(
    mut log: impl Write + Clone + Send + 'static,
    f: impl FnOnce(u16) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    util::setup_logger();

    let workdir = tempdir()?;
    let _ = writeln!(log, "running offsets() in {}", std::env::current_dir()?.display());
    let mut process = run("actyx-linux")?
        .current_dir(workdir.path())
        .stderr(Stdio::piped())
        .args(&["--bind-api=0", "--bind-admin=0", "--bind-swarm=0"])
        .spawn()?;
    let stderr = process.stderr.take().unwrap();

    // ensure that the test ends at some point
    let (tx, rx) = channel::<()>();
    let mut rx = Some((rx, process));

    let mut lines = BufReader::new(stderr).lines();
    let mut api = 0u16;
    for line in &mut lines {
        if let Some((rx, mut process)) = rx.take() {
            // unfortunately escargot doesn’t inform us when building is finished,
            // so we start the Actyx timeout upon seeing the first line of output
            spawn(move || {
                // timeout needs to allow for ax build time as well
                let _ = rx.recv_timeout(Duration::from_secs(120));
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
    let handle = spawn(move || {
        for line in lines {
            if let Ok(line) = line {
                let _ = writeln!(log2, "line: {}", line);
            }
        }
    });

    // let things settle - should perhaps wait for discovery events to be emitted
    std::thread::sleep(Duration::from_millis(300));

    // run the test
    let result = f(api);

    let _ = writeln!(log, "killing process");
    let _ = tx.send(());
    let _ = handle.join();
    result
}

fn get(json: &Value, ptr: &str) -> anyhow::Result<Value> {
    json.pointer(ptr).cloned().ok_or_else(|| anyhow!("not found"))
}

#[test]
fn offsets() -> anyhow::Result<()> {
    let log = Log::default();
    let result = with_api(log.clone(), |api| {
        let out = run("ax")?
            .args(&["-j", "events", "offsets", &format!("localhost:{}", api)])
            .output()?;
        eprintln!(
            "out:\n{}\nerr:\n{}\n---",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        ensure!(out.status.success());
        let json = serde_json::from_slice::<Value>(&out.stdout)?;
        ensure!(get(&json, "/code")? == json!("OK"), "line {} was: {}", line!(), json);
        let stream = get(&json, "/code")?
            .as_object()
            .v("result map")?
            .keys()
            .next()
            .cloned()
            .v("first key")?;

        let out = run("ax")?
            .args(&["events", "offsets", &format!("localhost:{}", api)])
            .output()?;
        eprintln!(
            "out:\n{}\nerr:\n{}\n---",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        ensure!(out.status.success());
        let out = String::from_utf8(out.stdout)?;
        ensure!(out.contains(&stream));
        Ok(())
    });
    if result.is_err() {
        eprintln!("{}", log);
    }
    result
}

#[test]
fn query() -> anyhow::Result<()> {
    let log = Log::default();
    let result = with_api(log.clone(), |api| {
        let out = run("ax")?
            .args(&["events", "query", &format!("localhost:{}", api), "FROM 'discovery' END"])
            .output()?;
        eprintln!(
            "out:\n{}\nerr:\n{}\n---",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        ensure!(out.status.success());

        let mut found = false;
        for line in String::from_utf8(out.stdout)?.split('\n') {
            if line.is_empty() {
                continue;
            }
            let json = serde_json::from_str::<Value>(line)?;
            ensure!(
                get(&json, "/appId")? == json!("com.actyx"),
                "line {} was: {}",
                line!(),
                json
            );
            found = true;
        }
        ensure!(found, "no events with text output");

        let out = run("ax")?
            .args(&[
                "-j",
                "events",
                "query",
                &format!("localhost:{}", api),
                "FROM 'discovery' END",
            ])
            .output()?;
        eprintln!(
            "out:\n{}\nerr:\n{}\n---",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        ensure!(out.status.success());

        let mut found = false;
        for line in String::from_utf8(out.stdout)?.split('\n') {
            if line.is_empty() {
                continue;
            }
            let json = serde_json::from_str::<Value>(line)?;
            ensure!(get(&json, "/code")? == json!("OK"), "line {} was: {}", line!(), json);
            ensure!(
                get(&json, "/result/appId")? == json!("com.actyx"),
                "line {} was: {}",
                line!(),
                json
            );
            found = true;
        }
        ensure!(found, "no events with json output");

        Ok(())
    });
    if result.is_err() {
        eprintln!("{}", log);
    }
    result
}
