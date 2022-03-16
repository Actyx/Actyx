use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn find_commit(prefix: &str, commit: String) -> Option<String> {
    let mut here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.push("..");
    here.push("..");
    here.push("..");
    here.push("versions");
    let versions = File::open(&here).expect("versions file");
    for line in BufReader::new(versions).lines() {
        let line = line.expect("valid versions line");
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let mut items = line.trim().split_whitespace();
        let version = items.next().expect("version");
        let hash = items.next().expect("hash");
        if version.starts_with(prefix) && hash == &*commit {
            return Some(version[prefix.len()..].to_owned());
        }
    }
    None
}

struct Version {
    version: String,
    git_hash: String,
}
fn get_version(env_var: &str) -> Version {
    match env::var(env_var) {
        Ok(v) => {
            let components: Vec<&str> = v.split('-').collect();

            if components.len() != 2 {
                panic!(
                    "Wrong format for ACTYX_VERSION. Should be \"<version>-<commit>\" is \"{}\"",
                    v
                );
            }

            Version {
                version: components[0].to_string(),
                git_hash: components[1].to_string(),
            }
        }

        Err(_) => {
            let mut history = Command::new("git")
                .args(["log", "--format=%H"])
                .stdout(Stdio::piped())
                .spawn()
                .expect("Error running git log --format=%H");
            let reader = BufReader::new(history.stdout.take().expect("stdout was piped"));
            let mut version = None;
            let prefix = if env_var == "ACTYX_VERSION" { "actyx-" } else { "cli-" };
            for line in reader.lines() {
                let line = line.expect("a single git hash");
                version = find_commit(prefix, line);
                if version.is_some() {
                    break;
                }
            }
            let version = format!("{}_dev", version.as_deref().unwrap_or("0.0.0"));

            // https://stackoverflow.com/a/5737794/576180
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

            println!("cargo:warning={} not set. Using \"{}-{}\"", env_var, version, git_hash);
            Version { version, git_hash }
        }
    }
}
fn main() {
    let Version {
        version: actyx_version,
        git_hash,
    } = get_version("ACTYX_VERSION");
    let Version {
        version: actyx_cli_version,
        ..
    } = get_version("ACTYX_VERSION_CLI");

    let profile = env::var("PROFILE").expect("PROFILE not set");
    let arch = env::var("TARGET")
        .expect("TARGET not set")
        .split('-')
        .next()
        .expect("TARGET has the wrong format")
        .to_string();

    println!("cargo:rustc-env=AX_VERSION={}", actyx_version);
    println!("cargo:rustc-env=AX_CLI_VERSION={}", actyx_cli_version);
    println!("cargo:rustc-env=AX_GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=AX_PROFILE={}", profile);
    println!("cargo:rerun-if-env-changed=ACTYX_VERSION");
    println!("cargo:rerun-if-env-changed=ACTYX_VERSION_CLI");
    println!("cargo:rerun-if-changed={}", get_common_git_dir().display());

    // Since target_arch armv7 does not exist, we add our own cfg parameter
    println!("cargo:rustc-cfg=AX_ARCH=\"{}\"", arch);
}

fn get_common_git_dir() -> PathBuf {
    let out = Command::new("git")
        .arg("rev-parse")
        .arg("--git-common-dir")
        .output()
        .expect("Error running git rev-parse --git-common-dir")
        .stdout;

    Path::new(String::from_utf8_lossy(&out).trim()).join("refs/heads")
}
