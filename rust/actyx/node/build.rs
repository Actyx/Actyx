fn main() {
    let version = util::version::NodeVersion::get();
    println!("cargo:rustc-env=AX_VERSION={}", version);
    println!("cargo:rerun-if-env-changed=ACTYX_VERSION");
}
