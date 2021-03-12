fn main() {
    if std::env::var("CARGO_CFG_TARGET_FAMILY").unwrap() == "windows" {
        let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        println!("cargo:rustc-link-search=native={}/res", dir);
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap();
        match target_env.as_str() {
            "gnu" => println!("cargo:rustc-link-lib=dylib=eventmsgs_gnu"),
            "msvc" => println!("cargo:rustc-link-lib=dylib=eventmsgs_msvc"),
            _ => panic!("Unsupported env: {}", target_env),
        };
    }
}
