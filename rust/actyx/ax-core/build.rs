fn main() {
    let arch = std::env::var("TARGET")
        .expect("TARGET not set")
        .split('-')
        .next()
        .expect("TARGET should have the right format")
        .to_string();

    // Since target_arch armv7 does not exist, we add our own cfg parameter
    println!("cargo:rustc-cfg=AX_ARCH=\"{}\"", arch);
}
