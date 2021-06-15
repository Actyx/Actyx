use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::collections::HashMap;
use std::{path::PathBuf, process::Command};
use util::version::NodeVersion;

fn get_commands() -> HashMap<&'static str, Vec<&'static str>> {
    let nodes = vec!["ls"];
    let settings = vec!["set", "get", "scopes", "unset", "schema"];
    let swarms = vec!["keygen"];
    vec![("nodes", nodes), ("settings", settings), ("swarms", swarms)]
        .into_iter()
        .collect()
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
        "[ERR_FILE_EXISTS] Error: File {} already exists in the specified path. Specify a different file name or path.\n",
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
    let expected = format!(
        "[ERR_PATH_INVALID] Error: Path \"{}\" does not exist. Specify an existing path.\n",
        identity_path
    );
    cli()
        .args(&["nodes", "ls", "--local", "localhost", "-i", &*identity_path])
        .assert()
        .stdout(predicate::str::contains(expected))
        .failure();
}

#[test]
fn internal_subcommand() {
    cli().args(&["internal", "help"]).assert().failure();
    cli()
        .env("HERE_BE_DRAGONS", "wrong")
        .args(&["internal", "help"])
        .assert()
        .failure();
    cli()
        .env("HERE_BE_DRAGONS", "zøg")
        .args(&["internal", "help"])
        .assert()
        .success();
    cli()
        .env("HERE_BE_DRAGONS", "zoeg")
        .args(&["internal", "help"])
        .assert()
        .success();
}

#[test]
fn version() {
    cli()
        .arg("--version")
        .assert()
        .stdout(format!("Actyx CLI {}\n", NodeVersion::get()))
        .success();
    cli()
        .arg("nodes")
        .assert()
        .stderr(predicate::str::starts_with("ax-nodes \n"))
        .failure();
    cli()
        .env("HERE_BE_DRAGONS", "zøg")
        .args(&["internal", "--help"])
        .assert()
        .stdout(predicate::str::starts_with("ax-internal \n"))
        .success();
    cli()
        .env("HERE_BE_DRAGONS", "zøg")
        .args(&["internal", "trees"])
        .assert()
        .stderr(predicate::str::starts_with("ax-internal-trees \n"))
        .failure();
}
