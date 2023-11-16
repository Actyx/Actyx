use assert_cmd::prelude::*;
use axlib::util::version::NodeVersion;
use predicates::str::starts_with;
use std::process::Command;

#[test]
fn version() {
    Command::cargo_bin("ax")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(starts_with(format!("ax {}\n", NodeVersion::get())));
}
