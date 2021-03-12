use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::sync::mpsc;
use std::thread;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
};
use std::{path::Path, process::Stdio};
use std::{path::PathBuf, process::Command};
use tempfile::tempdir;

fn get_commands() -> HashMap<&'static str, Vec<&'static str>> {
    let logs = vec!["tail"];
    let nodes = vec!["ls"];
    let settings = vec!["set", "get", "scopes", "unset", "schema"];
    let swarms = vec!["keygen"];
    vec![
        ("logs", logs),
        ("nodes", nodes),
        ("settings", settings),
        ("swarms", swarms),
    ]
    .into_iter()
    .collect()
}

fn get_axosnode_cli(cwd: &Path) -> Command {
    let mut cmd = Command::cargo_bin("actyx-linux").unwrap();
    cmd.args(&[
        "--bind-admin",
        "0",
        "--bind-api",
        "0",
        "--bind-swarm",
        "0",
        "--working-dir",
        &*cwd.display().to_string(),
    ]);
    cmd
}

fn cli() -> Command {
    Command::cargo_bin("ax").unwrap()
}

#[test]
fn cli_help() {
    // Check existence of commands:
    let commands = get_commands();
    commands.into_iter().for_each(|(domain, subcommand)| {
        subcommand.into_iter().for_each(|subcom| {
            let args = vec![domain, subcom, "--help"];
            cli().args(args).assert().success();
        });
    });
}

#[test]
fn cli_swarm_keygen() {
    cli().args(&["swarms", "keygen"]).assert().success();
}

#[test]
fn cli_users_keygen() {
    let temp_dir = tempfile::tempdir().unwrap();
    let identity_path = temp_dir.path().join("id").display().to_string();
    let expected = format!(
        r"Your private key has been saved at {}
Your public key has been saved at {}.pub",
        identity_path, identity_path,
    );
    cli()
        .args(&["users", "keygen", "--output", &*identity_path])
        .assert()
        .stdout(predicate::str::contains(expected))
        .stderr(predicate::eq("Generating public/private key pair ..\n"))
        .success();

    let file = PathBuf::from(identity_path);
    assert!(file.exists());
    assert!(file.with_extension("pub").exists());
}

#[test]
fn cli_users_keygen_err_on_existing_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file = temp_dir.path().join("id");
    let identity_path = file.display().to_string();
    std::fs::write(file, "yay").unwrap();
    let expected = format!(
        "[ERR_FILE_EXISTS] Error: File {} already exits in the specified path. Specify a different file name or path.\n",
        identity_path
    );
    cli()
        .args(&["users", "keygen", "--output", &*identity_path])
        .assert()
        .stdout(predicate::str::contains(expected))
        .stderr(predicate::eq("Generating public/private key pair ..\n"))
        .failure();
}

#[test]
fn cli_fail_on_missing_identity() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file = temp_dir.path().join("id");
    let identity_path = file.display().to_string();
    let expected = format!("[ERR_PATH_INVALID] Error: Path \"{}\" does not exist. Specify an existing path. (No such file or directory (os error 2))\n", identity_path);
    cli()
        .args(&["nodes", "ls", "--local", "localhost", "-i", &*identity_path])
        .assert()
        .stdout(predicate::str::contains(expected))
        .failure();
}

/*
This test is ignored because we don't want to run them every time in the
usual CI process, because of its dependency on the `actyx-linux` bin target.
To succesfully run these integration tests, you'll need:
```
    cargo build --bin actyx-linux
    RUST_LOG=trace cargo --locked test --color=always --package actyx-cli --test cli -- --exact --ignored
```
*/
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn test_logging() {
    let temp_dir = tempdir().unwrap();

    // Wrong endpoint:
    cli()
        .args(&["logs", "tail", "--local", "localhost:12345"])
        .current_dir(&temp_dir)
        .assert()
        .failure();

    let mut axosnode = get_axosnode_cli(temp_dir.path());
    let mut axosnode_handle = axosnode.stdout(Stdio::piped()).spawn().unwrap();
    let admin_port: u16 = {
        let regex = regex::Regex::new(r"(?:ADMIN_API_BOUND: Admin API bound to /ip4/127.0.0.1/tcp/)(\d*)").unwrap();
        let mut buf_reader = BufReader::new(axosnode_handle.stdout.as_mut().unwrap());
        let mut buf = String::new();
        loop {
            if buf_reader.read_line(&mut buf).is_ok() {
                if let Some(x) = regex.captures(&*buf).and_then(|c| c.get(1).map(|x| x.as_str())) {
                    break x.parse().unwrap();
                }
            } else {
                panic!("Actyx Node didn't start up")
            }
        }
    };
    let axosnode_bind = format!("localhost:{}", admin_port);
    let (sender, receiver) = mpsc::sync_channel(0);
    let handle = thread::spawn(move || {
        let _ = receiver.recv();
        axosnode_handle.kill().expect("Server exited before kill");
        axosnode_handle.wait().expect("Wait failed");
    });

    println!("Node started");

    let identity_path = temp_dir.path().join("id").display().to_string();
    cli()
        .args(&["users", "keygen", "--output", &*identity_path])
        .assert()
        .success();
    // And now logs!
    cli()
        .args(&[
            "logs",
            "tail",
            "--local",
            "-n",
            "100",
            &*axosnode_bind,
            "-i",
            &*identity_path,
        ])
        .current_dir(&temp_dir)
        .assert()
        .append_context("logscvd", "read one log back")
        .stdout(predicate::str::contains("API_BOUND"))
        .success();

    // Same logs, but in json format:
    cli()
        .args(&[
            "-j",
            "logs",
            "tail",
            "--local",
            "-n",
            "100",
            &*axosnode_bind,
            "-i",
            &*identity_path,
        ])
        .current_dir(&temp_dir)
        .assert()
        .append_context("logscvd", "getting the same log but json format")
        // This is available only in the json format:
        .stdout(predicate::str::contains("API_BOUND"));

    // All entries:
    cli()
        .args(&[
            "logs",
            "tail",
            "--local",
            "--all-entries",
            &*axosnode_bind,
            "-i",
            &*identity_path,
        ])
        .current_dir(&temp_dir)
        .assert()
        .append_context("logscvd", "read one log back")
        .stdout(predicate::str::contains("API_BOUND"))
        .success();

    // Stop logscvd
    sender.send(()).unwrap();
    handle.join().unwrap();
}
