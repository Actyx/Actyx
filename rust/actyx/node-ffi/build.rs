fn main() {
    println!("cargo:rerun-if-env-changed=ANDROID6");
    if std::env::var("ANDROID6").is_ok() {
        cc::Build::new()
            .cargo_metadata(false)
            .include("android_headers")
            .file("ifaddrs.c")
            .compile("ifaddrs");
        println!("cargo:rustc-link-lib=static:+whole-archive=ifaddrs");
        println!(
            "cargo:rustc-link-search=native={}",
            std::env::var("OUT_DIR").expect("cannot build in non-UTF8 target directory")
        );
    }
}
