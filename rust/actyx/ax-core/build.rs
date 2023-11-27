use std::process::Command;

fn git_hash() -> String {
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

    format!("{}{}", hash, dirty)
}

fn main() {
    let arch = std::env::var("TARGET")
        .expect("TARGET not set")
        .split('-')
        .next()
        .expect("TARGET should have the right format")
        .to_string();

    // Since target_arch armv7 does not exist, we add our own cfg parameter
    println!("cargo:rustc-cfg=AX_ARCH=\"{}\"", arch);
    println!("cargo:rustc-env=AX_GIT_HASH={}", git_hash());
}
