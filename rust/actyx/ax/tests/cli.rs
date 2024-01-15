use assert_cmd::{assert::OutputAssertExt, cargo::CommandCargoExt};
use ax_core::util::version::NodeVersion;
use maplit::btreemap;
use predicates::{prelude::predicate, str::starts_with};
use std::{collections::HashMap, path::PathBuf, process::Command};

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
fn cli_version() {
    let mut node_version = NodeVersion::get().clone();
    node_version.version = env!("CARGO_PKG_VERSION").to_string();
    cli()
        .arg("--version")
        .assert()
        .success()
        .stdout(starts_with(format!("ax {}\n", node_version)));
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
    cli().args(["swarms", "keygen"]).assert().success();
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
        .args(["users", "keygen", "--output", &*identity_path])
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
        .args(["users", "keygen", "--output", &*identity_path])
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
        .args(["nodes", "ls", "localhost", "-i", &*identity_path])
        .assert()
        .stderr(predicate::str::contains(expected))
        .failure();
}

#[test]
fn internal_subcommand() {
    use predicate::str::contains;
    use predicates::prelude::*;
    cli()
        .env("HERE_BE_DRAGONS", "wrong")
        .assert()
        .failure()
        .stderr(contains("internal").not());
    cli()
        .args(["internal"])
        .assert()
        .failure()
        .stderr(contains("do not use until instructed by Actyx"));
    cli()
        .env("HERE_BE_DRAGONS", "zøg")
        .assert()
        .failure()
        .stderr(contains("internal"));
    cli()
        .env("HERE_BE_DRAGONS", "zoeg")
        .assert()
        .failure()
        .stderr(contains("internal"));
}

#[test]
fn version() {
    let mut node_version = NodeVersion::get().clone();
    node_version.version = env!("CARGO_PKG_VERSION").to_string();
    let first_line = format!("ax {}\n", node_version);
    cli().arg("--version").assert().stdout(first_line).success();

    #[derive(PartialEq)]
    enum Type {
        Branch,
        Leaf,
    }
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
        vec!["settings", "local"] => Branch,
        vec!["settings", "local", "get"] => Leaf,
        vec!["settings", "local", "set"] => Leaf,
        vec!["settings", "local", "unset"] => Leaf,
        vec!["settings", "local", "init"] => Leaf,
        vec!["swarms"] => Branch,
        vec!["swarms", "keygen"] => Leaf,
        vec!["users"] => Branch,
        vec!["users", "keygen"] => Leaf,
        vec!["users", "add-key"] => Leaf,
    };

    let first_line = |sub| format!("ax-{} {}\n", sub, node_version);

    for (args, tpe) in commands {
        if tpe == Branch {
            cli()
                .args(args.iter().chain(&["--version"]))
                .env("HERE_BE_DRAGONS", "zøg")
                .assert()
                .success()
                .stdout(starts_with(&*first_line(args.join("-"))));
        }
        cli()
            .args(&*args.into_iter().chain(once("--help")).collect::<Vec<_>>())
            .env("HERE_BE_DRAGONS", "zøg")
            .assert()
            .success();
    }
}

// calls `ax settings local [..args] --working-dir [working_dir]`
fn call_cli_settings_local<I, S>(working_dir: impl Into<String>, args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let args = ["settings", "local"]
        .into_iter()
        .map(String::from)
        .chain(args.into_iter().map(|x| String::from(x.as_ref())))
        .chain([String::from("--working-dir"), working_dir.into()])
        .collect::<Vec<_>>();

    let mut proc = cli();
    proc.args(args);
    proc
}

#[test]
fn cli_settings_local() {
    let temp_dir = String::from(tempfile::tempdir().unwrap().path().to_str().unwrap());

    // Init
    {
        // First --fail-on-existing init to empty dir should success
        call_cli_settings_local(&temp_dir, ["init", "--fail-on-existing"])
            .assert()
            .success();
        // Second --fail-on-existing init to the same dir should fail
        call_cli_settings_local(&temp_dir, ["init", "--fail-on-existing"])
            .assert()
            .failure();
        // Without --fail-on-existing, init to the same dir should success
        call_cli_settings_local(&temp_dir, ["init"]).assert().success();
    }

    // Get/Set/Unset
    {
        // Get should succeed after init
        call_cli_settings_local(&temp_dir, ["get", "/admin/displayName"])
            .assert()
            .success();

        // Test setting new display name
        let new_display_name = "new_display_name";
        call_cli_settings_local(&temp_dir, ["set", "/admin/displayName", new_display_name])
            .assert()
            .success();
        call_cli_settings_local(&temp_dir, ["get", "/admin/displayName"])
            .assert()
            .stdout(predicate::str::contains(new_display_name))
            .success();

        // Unsetting resets displayName back to "Default Node"
        call_cli_settings_local(&temp_dir, ["unset", "/admin/displayName"])
            .assert()
            .success();
        call_cli_settings_local(&temp_dir, ["get", "/admin/displayName"])
            .assert()
            .stdout(predicate::str::contains("Default Node"))
            .success();
    }
}

#[test]
fn cli_settings_local_empty_dir() {
    let empty_dir = String::from(tempfile::tempdir().unwrap().path().to_str().unwrap());
    // Get should fail on empty_dir
    call_cli_settings_local(&empty_dir, ["get", "/admin/displayName"])
        .assert()
        .failure();
    call_cli_settings_local(&empty_dir, ["set", "/admin/displayName", ""])
        .assert()
        .failure();
    call_cli_settings_local(&empty_dir, ["unset", "/admin/displayName"])
        .assert()
        .failure();
}
