// This build script propagates the proper `TARGET` information for
// https://doc.rust-lang.org/cargo/appendix/glossary.html#target
fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").expect("cargo should have set this variable");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").expect("cargo should have set this variable");
    println!("cargo:rustc-env=TARGET_ARCH={}", target_arch);
    println!("cargo:rustc-env=TARGET_OS={}", target_os);
}
