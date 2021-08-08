use std::{env, process::Command};
use util::build::add_icon_to_bin_when_building_for_win;

fn main() {
    add_icon_to_bin_when_building_for_win("./assets/actyxcli.ico");

    let (version, git_hash) = if let Ok(v) = env::var("ACTYX_VERSION_CLI") {
        let mut components = v.split('-');
        let version = components
            .next()
            .expect("ACTYX_VERSION_CLI needs to have shape <ver>-<hash>");
        let hash = components
            .next()
            .expect("ACTYX_VERSION_CLI needs to have shape <ver>-<hash>");
        (version.to_string(), hash.to_string())
    } else {
        let out = Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .output()
            .expect("Error running git status --porcelain");

        let dirty = if out.stdout.is_empty() { "" } else { "_dirty" };

        let git_head_out = Command::new("git")
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .expect("Error running git rev-parse --short HEAD");

        let hash = String::from_utf8_lossy(&git_head_out.stdout).trim().to_string();

        let git_hash = format!("{}{}", hash, dirty);
        ("unknown".to_string(), git_hash)
    };

    let profile = env::var("PROFILE").expect("PROFILE not set");
    let target = env::var("TARGET").expect("TARGET not set");
    let mut target = target.split('-');
    let arch = target.next().expect(&*format!("no target triple: {}", profile));
    let _vendor = target.next().expect(&*format!("no target triple: {}", profile));
    let mut os = target.next().expect(&*format!("no target triple: {}", profile));
    if os == "darwin" {
        os = "macos";
    }

    println!(
        "cargo:rustc-env=AX_CLI_VERSION={}-{}-{}-{}-{}",
        version, git_hash, os, arch, profile
    );
    println!("cargo:rerun-if-env-changed=ACTYX_VERSION_CLI");
}
