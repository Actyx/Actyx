use regex::Regex;
use std::env;
use std::process::Command;

fn main() {
    let (version, git_hash) = match env::var("ACTYX_VERSION") {
        Ok(v) => {
            let components: Vec<&str> = v.split('-').collect();

            if components.len() != 2 {
                panic!(
                    "Wrong format for ACTYX_VERSION. Should be \"<version>-<commit>\" is \"{}\"",
                    v
                );
            }

            (components[0].to_string(), components[1].to_string())
        }

        Err(_) => {
            // Fall back to using git if the environment variable is not provided.
            // We shell out to the git command instead of using git2 because for these purposes the git2 API is too annoying
            let tag_regex = Regex::new("^actyx-([0-9+]\\.[0-9+]\\.[0-9+](-.*)?)$").unwrap();

            let out = Command::new("git").arg("tag").output().expect("Error running git tag");

            let ver = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| tag_regex.is_match(l))
                .last()
                .and_then(|tag| tag_regex.captures(tag))
                .and_then(|c: regex::Captures| c.get(1))
                .map(|m| m.as_str())
                .unwrap_or("unknown")
                .to_string();

            // https://stackoverflow.com/a/5737794/576180
            let out = Command::new("git")
                .arg("status")
                .arg("--porcelain")
                .output()
                .expect("Error running git status --porcelain");

            let dirty = if out.stdout.is_empty() { "" } else { "_dirty" };

            let out = Command::new("git")
                .arg("rev-parse")
                .arg("--short")
                .arg("HEAD")
                .output()
                .expect("Error running git rev-parse --short HEAD");

            let hash = String::from_utf8_lossy(&out.stdout).trim().to_string();

            let git_hash = format!("{}{}", hash, dirty);

            (ver, git_hash)
        }
    };

    let target = env::var("TARGET").expect("TARGET not set").replace("-", "_");

    let profile = env::var("PROFILE").expect("PROFILE not set");

    println!("cargo:rustc-env=AX_VERSION={}", version);
    println!("cargo:rustc-env=AX_GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=AX_PROFILE={}", profile);
}
