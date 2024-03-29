use anyhow::{anyhow, bail, ensure};
use escargot::{format::Message, CargoBuild};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fmt::Write,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
    sync::{mpsc::channel, Arc, Once},
    thread::spawn,
    time::{Duration, Instant},
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

fn setup() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // This makes the path consistent, it works since its the same project we're building using escargot
        let cargo_path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
        // build needed binaries for quicker execution
        for bin in &["ax"] {
            eprintln!("building {}", bin);
            for msg in CargoBuild::new().manifest_path(cargo_path).bin(*bin).exec().unwrap() {
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
    });
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
    let cargo_path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    Ok(CargoBuild::new().manifest_path(cargo_path).bin(bin).run()?.command())
}

fn with_api(
    mut log: impl Write + Clone + Send + 'static,
    f: impl FnOnce(u16, &Path) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    ax_core::util::setup_logger();
    setup();

    let workdir = tempdir()?;

    let _ = writeln!(log, "running AX in {}", std::env::current_dir()?.display());
    let mut process = run("ax")?
        .args(["run"])
        .current_dir(workdir.path())
        .stderr(Stdio::piped())
        .args(["--bind-api=0", "--bind-admin=0", "--bind-swarm=0"])
        .env("RUST_LOG", "debug")
        .spawn()?;
    let stderr = process.stderr.take().unwrap();

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
    let (tx, rx) = channel::<()>();
    let mut rx = Some((rx, process));

    let mut lines = BufReader::new(stderr).lines();
    let mut api = 0u16;
    for line in &mut lines {
        if let Some((rx, mut process)) = rx.take() {
            // unfortunately escargot doesn’t inform us when building is finished,
            // so we start the AX timeout upon seeing the first line of output
            spawn(move || {
                let _ = rx.recv_timeout(Duration::from_secs(60));
                eprintln!("killing AX");
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
        for line in lines.flatten() {
            let _ = writeln!(log2, "line: {}", line);
        }
    });

    let started = Instant::now();
    loop {
        let offsets = match get_offsets(api, identity.as_ref()) {
            Ok(o) => o,
            Err(e) => {
                if started.elapsed() > Duration::from_secs(5) {
                    return Err(e);
                } else {
                    continue;
                }
            }
        };
        if get(&offsets, "/code")? == json!("OK")
            && !get(&offsets, "/result/present")?
                .as_object()
                .v("result map")?
                .is_empty()
        {
            break;
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

fn get_offsets(api: u16, identity: &Path) -> anyhow::Result<Value> {
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
    eprintln!(
        "prep out:\n{}\nerr:\n{}\n---",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    ensure!(out.status.success());
    let v = serde_json::from_slice::<Value>(&out.stdout)?;
    ensure!(v.pointer("/code").is_some());
    Ok(v)
}

fn get(json: &Value, ptr: &str) -> anyhow::Result<Value> {
    json.pointer(ptr).cloned().ok_or_else(|| anyhow!("{} not found", ptr))
}
fn o(s: &str) -> &OsStr {
    OsStr::new(s)
}

#[test]
fn offsets() -> anyhow::Result<()> {
    let log = Log::default();
    let result = with_api(log.clone(), |api, identity| {
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
        eprintln!(
            "out:\n{}\nerr:\n{}\n---",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        ensure!(out.status.success());
        let json = serde_json::from_slice::<Value>(&out.stdout)?;
        ensure!(get(&json, "/code")? == json!("OK"), "line {} was: {}", line!(), json);
        let stream = get(&json, "/result/present")?
            .as_object()
            .v("result map")?
            .keys()
            .next()
            .cloned()
            .v("first key")?;

        let out = run("ax")?
            .args([
                o("events"),
                o("offsets"),
                o("-i"),
                identity.as_os_str(),
                o(&format!("127.0.0.1:{}", api)),
            ])
            .output()?;
        eprintln!(
            "out:\n{}\nerr:\n{}\n---",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        ensure!(out.status.success());
        let out = String::from_utf8(out.stdout)?;
        ensure!(out.contains(&stream), "{}", out);
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
    let result = with_api(log.clone(), |api, identity| {
        let out = run("ax")?
            .args([
                o("events"),
                o("query"),
                o("-i"),
                identity.as_os_str(),
                o(&format!("127.0.0.1:{}", api)),
                o("FROM 'discovery' END"),
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
            let start = line.find(": ").ok_or_else(|| anyhow!("cannot parse"))? + 2;
            let json = serde_json::from_str::<Value>(&line[start..])?;
            get(&json, "/NewListenAddr")
                .or_else(|_| get(&json, "/NewObservedAddr"))
                .or_else(|_| get(&json, "/ExpiredObservedAddr"))?;
            found = true;
        }
        ensure!(found, "no events with text output");

        let out = run("ax")?
            .args([
                o("events"),
                o("query"),
                o("-ji"),
                identity.as_os_str(),
                o(&format!("127.0.0.1:{}", api)),
                o("FROM 'discovery' END"),
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
            ensure!(
                get(&json, "/appId")? == json!("com.actyx"),
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

#[test]
fn bad_query() -> anyhow::Result<()> {
    let log = Log::default();
    let result = with_api(log.clone(), |api, identity| {
        let out = run("ax")?
            .args([
                o("events"),
                o("query"),
                o("-i"),
                identity.as_os_str(),
                o(&format!("127.0.0.1:{}", api)),
                o("FROM [1] END"),
            ])
            .output()?;
        eprintln!(
            "out:\n{}\nerr:\n{}\n---",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        ensure!(!out.status.success());
        let out = String::from_utf8(out.stderr)?;
        ensure!(
            out == "[ERR_INVALID_INPUT] Error: The query uses beta features that are not enabled: fromArray.\n",
            "{}",
            out
        );

        let out = run("ax")?
            .args([
                o("events"),
                o("query"),
                o("-ji"),
                identity.as_os_str(),
                o(&format!("127.0.0.1:{}", api)),
                o("FROM [1] END"),
            ])
            .output()?;
        eprintln!(
            "out:\n{}\nerr:\n{}\n---",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        ensure!(!out.status.success());
        let out = String::from_utf8(out.stdout)?;
        ensure!(
            out == r#"{"code":"ERR_INVALID_INPUT","message":"The query uses beta features that are not enabled: fromArray."}
"#,
            "{}",
            out
        );

        Ok(())
    });
    if result.is_err() {
        eprintln!("{}", log);
    }
    result
}

#[test]
fn publish() -> anyhow::Result<()> {
    let log = Log::default();
    let result = with_api(log.clone(), |api, identity| {
        let out = run("ax")?
            .args([
                o("events"),
                o("publish"),
                o("-ji"),
                identity.as_os_str(),
                o(&format!("127.0.0.1:{}", api)),
                o(r#"{ "baz":42 }"#),
                o("-t"),
                o("foo"),
                o("-t"),
                o("bar"),
            ])
            .output()?;
        eprintln!(
            "out:\n{}\nerr:\n{}\n---",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        ensure!(out.status.success());
        let json = serde_json::from_slice::<Value>(&out.stdout)?;
        ensure!(get(&json, "/code")? == json!("OK"), "line {} was: {}", line!(), json);
        Ok(())
    });
    if result.is_err() {
        eprintln!("{}", log);
    }
    result
}

#[test]
fn diagnostics() -> anyhow::Result<()> {
    let log = Log::default();
    let result = with_api(log.clone(), |api, identity| {
        let out = run("ax")?
            .args([
                o("events"),
                o("query"),
                o("-i"),
                identity.as_os_str(),
                o(&format!("127.0.0.1:{}", api)),
                o("FROM 'discovery' SELECT _ - 3"),
            ])
            .output()?;
        eprintln!(
            "out:\n{}\nerr:\n{}\n---",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        ensure!(out.status.success());
        let out = String::from_utf8(out.stdout)?;
        ensure!(out.contains("is not of type Number"), "{}", out);
        Ok(())
    });
    if result.is_err() {
        eprintln!("{}", log);
    }
    result
}

#[test]
fn aggregate() -> anyhow::Result<()> {
    let log = Log::default();
    let result = with_api(log.clone(), |api, identity| {
        let out = run("ax")?
            .args([
                o("events"),
                o("query"),
                o("-ji"),
                identity.as_os_str(),
                o(&format!("127.0.0.1:{}", api)),
                o("FEATURES(zøg aggregate) FROM 'discovery' AGGREGATE SUM(1)"),
            ])
            .output()?;
        eprintln!(
            "out:\n{}\nerr:\n{}\n---",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        ensure!(out.status.success());
        let json = serde_json::from_slice::<Value>(&out.stdout)?;
        ensure!(
            get(&json, "/payload")?.as_u64() > Some(0),
            "{:?}",
            get(&json, "/payload")?.as_u64()
        );
        Ok(())
    });
    if result.is_err() {
        eprintln!("{}", log);
    }
    result
}

#[test]
fn topic_delete() -> anyhow::Result<()> {
    let log = Log::default();
    let result = with_api(log.clone(), |api, identity| {
        // Change the topic
        let out = run("ax")?
            .args([
                o("settings"),
                o("set"),
                o("-ji"),
                identity.as_os_str(),
                o("/swarm"),
                o("{\"topic\": \"new_topic\"}"),
                o(&format!("127.0.0.1:{}", api)),
            ])
            .env("RUST_LOG", "debug")
            .output()?;
        assert!(out.status.success());

        // List both topics
        let out = run("ax")?
            .args([
                o("topics"),
                o("ls"),
                o("-ji"),
                identity.as_os_str(),
                o(&format!("127.0.0.1:{}", api)),
            ])
            .env("RUST_LOG", "debug")
            .output()?;
        assert!(out.status.success());

        let json = serde_json::from_slice::<Value>(&out.stdout)?;
        assert!(get(&json, "/code")? == json!("OK"));
        assert!(get(&json, "/result/0/response/activeTopic")? == json!("new_topic"));

        // Size is not predictable so we're just checking the keys
        let topics = serde_json::from_value::<BTreeMap<String, u64>>(get(&json, "/result/0/response/topics")?)?;
        for (value, expected) in topics.keys().zip(&["default-topic", "new_topic"]) {
            assert!(value == expected);
        }

        // Delete the old topic
        let out = run("ax")?
            .args([
                o("topics"),
                o("delete"),
                o("default-topic"),
                o(&format!("127.0.0.1:{}", api)),
                o("-ji"),
                identity.as_os_str(),
            ])
            .env("RUST_LOG", "debug")
            .output()?;
        assert!(out.status.success());
        let json = serde_json::from_slice::<Value>(&out.stdout)?;
        assert!(get(&json, "/code")? == json!("OK"));
        assert!(get(&json, "/result/0/response/deleted")? == json!(true));

        // List again to compare
        let out = run("ax")?
            .args([
                o("topics"),
                o("ls"),
                o("-ji"),
                identity.as_os_str(),
                o(&format!("127.0.0.1:{}", api)),
            ])
            .env("RUST_LOG", "debug")
            .output()?;
        assert!(out.status.success());

        let json = serde_json::from_slice::<Value>(&out.stdout)?;
        assert!(get(&json, "/code")? == json!("OK"));
        assert!(get(&json, "/result/0/response/activeTopic")? == json!("new_topic"));

        let topics = serde_json::from_value::<BTreeMap<String, u64>>(get(&json, "/result/0/response/topics")?)?;
        assert!(topics.keys().next().unwrap() == "new_topic");

        Ok(())
    });
    if result.is_err() {
        eprintln!("{}", log);
    }
    result
}

#[test]
fn topic_delete_non_existing() -> anyhow::Result<()> {
    let log = Log::default();
    let result = with_api(log.clone(), |api, identity| {
        // Delete the old topic
        let out = run("ax")?
            .args([
                o("topics"),
                o("delete"),
                o("non-existing-topic"),
                o(&format!("127.0.0.1:{}", api)),
                o("-ji"),
                identity.as_os_str(),
            ])
            .env("RUST_LOG", "debug")
            .output()?;
        assert!(out.status.success());

        let json = serde_json::from_slice::<Value>(&out.stdout)?;
        assert!(get(&json, "/code")? == json!("OK"));
        assert!(get(&json, "/result/0/response/deleted")? == json!(false));
        Ok(())
    });
    if result.is_err() {
        eprintln!("{}", log);
    }
    result
}

#[test]
fn topic_delete_prefix() -> anyhow::Result<()> {
    let log = Log::default();
    let result = with_api(log.clone(), |api, identity| {
        // Change the topic to t-i
        let out = run("ax")?
            .args([
                o("settings"),
                o("set"),
                o("-ji"),
                identity.as_os_str(),
                o("/swarm"),
                o("{\"topic\": \"t-i\"}"),
                o(&format!("127.0.0.1:{}", api)),
            ])
            .env("RUST_LOG", "debug")
            .output()?;
        assert!(out.status.success());

        // Change the topic to t-index
        let out = run("ax")?
            .args([
                o("settings"),
                o("set"),
                o("-ji"),
                identity.as_os_str(),
                o("/swarm"),
                o("{\"topic\": \"t-index\"}"),
                o(&format!("127.0.0.1:{}", api)),
            ])
            .env("RUST_LOG", "debug")
            .output()?;
        assert!(out.status.success());

        // List the topics to gather some info
        let out = run("ax")?
            .args([
                o("topics"),
                o("ls"),
                o("-ji"),
                identity.as_os_str(),
                o(&format!("127.0.0.1:{}", api)),
            ])
            .env("RUST_LOG", "debug")
            .output()?;
        assert!(out.status.success());

        let json = serde_json::from_slice::<Value>(&out.stdout)?;
        assert!(get(&json, "/code")? == json!("OK"));
        assert!(get(&json, "/result/0/response/activeTopic")? == json!("t-index"));

        // Size is not predictable so we're just checking the keys
        let topics = serde_json::from_value::<BTreeMap<String, u64>>(get(&json, "/result/0/response/topics")?)?;
        for (value, expected) in topics.keys().zip(&["default-topic", "t-i", "t-index"]) {
            assert!(value == expected);
        }

        // Keep the t-index size around to ensure it doesnt shrink
        let t_index_size = serde_json::from_value::<u64>(get(&json, "/result/0/response/topics/t-index")?)?;

        // Delete the prefix topic
        let out = run("ax")?
            .args([
                o("topics"),
                o("delete"),
                o("t-i"),
                o(&format!("127.0.0.1:{}", api)),
                o("-ji"),
                identity.as_os_str(),
            ])
            .env("RUST_LOG", "debug")
            .output()?;
        assert!(out.status.success());
        let json = serde_json::from_slice::<Value>(&out.stdout)?;
        assert!(get(&json, "/code")? == json!("OK"));
        assert!(get(&json, "/result/0/response/deleted")? == json!(true));

        // List again to compare
        let out = run("ax")?
            .args([
                o("topics"),
                o("ls"),
                o("-ji"),
                identity.as_os_str(),
                o(&format!("127.0.0.1:{}", api)),
            ])
            .env("RUST_LOG", "debug")
            .output()?;
        assert!(out.status.success());

        let json = serde_json::from_slice::<Value>(&out.stdout)?;
        assert!(get(&json, "/code")? == json!("OK"));
        assert!(get(&json, "/result/0/response/activeTopic")? == json!("t-index"));

        let topics = serde_json::from_value::<BTreeMap<String, u64>>(get(&json, "/result/0/response/topics")?)?;
        for (value, expected) in topics.keys().zip(&["default-topic", "t-index"]) {
            assert!(value == expected);
        }

        // Ensure the topic size didn't shrink (i.e. we didnt accidentaly delete something "extra")
        assert!(*topics.get("t-index").unwrap() >= t_index_size);

        Ok(())
    });
    if result.is_err() {
        eprintln!("{}", log);
    }
    result
}
