fn main() {
    println!("cargo:rerun-if-env-changed=ANDROID6");
    if std::env::var("ANDROID6").is_ok() {
        cc::Build::new()
            .include("android_headers")
            .file("ifaddrs.c")
            .compile("ifaddrs");
    }
}
