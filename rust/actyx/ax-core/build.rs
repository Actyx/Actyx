use std::vec::Vec;

// We need this build script to propagate the target variables through compilation
// instead of relying on #[cfg(target_arch = "...")] which isn't always "guessable"
// this way, we don't guess, we use what Cargo uses, we split and propagate.
// See the following link for more information:
// https://doc.rust-lang.org/cargo/appendix/glossary.html#target
fn main() {
    let target = std::env::var("TARGET").expect("TARGET to be defined");
    let target = target.split('-').collect::<Vec<_>>();
    let target_arch = target[0];
    let target_sys = target[2];
    println!("cargo:rustc-env=TARGET_ARCH={}", target_arch);
    println!("cargo:rustc-env=TARGET_SYS={}", target_sys);
}
