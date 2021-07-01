use assert_cmd::prelude::*;
use predicates::str::starts_with;
use std::process::Command;
use util::version::NodeVersion;

#[test]
fn version() {
    Command::cargo_bin("actyx-linux")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(starts_with(format!("Actyx {}\n", NodeVersion::get())));
}
