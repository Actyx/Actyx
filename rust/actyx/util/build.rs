use std::env;
use std::process::Command;

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
            let version = "unknown".to_string();

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

    // Since target_arch armv7 does not exist, we add our own cfg parameter
    println!("cargo:rustc-cfg=AX_ARCH=\"{}\"", arch);
}
