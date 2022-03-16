use assert_cmd::prelude::*;
use maplit::btreemap;
use predicates::prelude::*;
use std::collections::HashMap;
use std::{path::PathBuf, process::Command};
use util::version::NodeVersion;

fn get_commands() -> HashMap<&'static str, Vec<&'static str>> {
    let apps = vec!["sign"];
    let events = vec!["offsets", "query", "publish", "dump", "restore"];
    let nodes = vec!["ls", "inspect"];
    let settings = vec!["set", "get", "unset", "schema"];
    let swarms = vec!["keygen"];
    let users = vec!["keygen", "add-key"];
    vec![
        ("apps", apps),
        ("events", events),
        ("nodes", nodes),
        ("settings", settings),
        ("swarms", swarms),
        ("users", users),
    ]
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
        .stderr(predicate::str::contains(expected))
        .stderr(predicate::str::contains("Generating public/private key pair ..\n"))
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
        .args(&["nodes", "ls", "localhost", "-i", &*identity_path])
        .assert()
        .stderr(predicate::str::contains(expected))
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
    let first_line = format!("Actyx CLI {}\n", NodeVersion::get_cli());
    cli().arg("--version").assert().stdout(first_line).success();

    #[derive(PartialEq)]
    enum Type {
        Branch,
        Leaf,
    }
    use predicate::str::starts_with;
    use std::iter::once;
    use Type::*;

    let commands = btreemap! {
        vec!["apps"] => Branch,
        vec!["apps", "sign"] => Leaf,
        vec!["events"] => Branch,
        vec!["events", "offsets"] => Leaf,
        vec!["events", "query"] => Leaf,
        vec!["events", "publish"] => Leaf,
        vec!["events", "dump"] => Leaf,
        vec!["events", "restore"] => Leaf,
        vec!["internal"] => Branch,
        vec!["internal", "convert"] => Leaf,
        vec!["internal", "trees"] => Branch,
        vec!["internal", "trees", "dump"] => Leaf,
        vec!["internal", "trees", "explore"] => Leaf,
        vec!["nodes"] => Branch,
        vec!["nodes", "inspect"] => Leaf,
        vec!["nodes", "ls"] => Leaf,
        vec!["settings"] => Branch,
        vec!["settings", "schema"] => Leaf,
        vec!["settings", "get"] => Leaf,
        vec!["settings", "set"] => Leaf,
        vec!["settings", "unset"] => Leaf,
        vec!["swarms"] => Branch,
        vec!["swarms", "keygen"] => Leaf,
        vec!["users"] => Branch,
        vec!["users", "keygen"] => Leaf,
        vec!["users", "add-key"] => Leaf,
    };

    let first_line = |sub| format!("ax-{} {}\n", sub, NodeVersion::get_cli());
    for (args, tpe) in commands {
        if tpe == Branch {
            cli()
                .args(&*args)
                .env("HERE_BE_DRAGONS", "zøg")
                .assert()
                .failure()
                .stderr(starts_with(&*first_line(args.join("-"))));
        }
        let name = args.join("-");
        cli()
            .args(&*args.into_iter().chain(once("--help")).collect::<Vec<_>>())
            .env("HERE_BE_DRAGONS", "zøg")
            .assert()
            .success()
            .stdout(starts_with(&*first_line(name)));
    }
}
