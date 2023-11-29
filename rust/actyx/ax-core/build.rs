// This build script propagates the proper `TARGET` information for
// https://doc.rust-lang.org/cargo/appendix/glossary.html#target
fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").expect("cargo should have set this variable");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").expect("cargo should have set this variable");
    let target_release_arch = match target_arch.as_str() {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        "arm" => "arm",
        "armv7" => "armhf",
        _ => unreachable!("unsupported architecture"),
    };
    println!("cargo:rustc-env=TARGET_ARCH={}", target_arch);
    println!("cargo:rustc-env=TARGET_RELEASE_ARCH={}", target_release_arch);
    println!("cargo:rustc-env=TARGET_OS={}", target_os);
}
