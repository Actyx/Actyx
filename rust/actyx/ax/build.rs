use std::env;
use winres::WindowsResource;

pub fn add_icon_to_bin_when_building_for_win(icon_path: &str) {
    if env::var("CARGO_CFG_TARGET_FAMILY").unwrap() == "windows" {
        let mut res = WindowsResource::new();
        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap();
        match target_env.as_str() {
            "gnu" => res
                .set_ar_path("x86_64-w64-mingw32-ar")
                .set_windres_path("x86_64-w64-mingw32-windres")
                .set_toolkit_path(".")
                .set_icon(icon_path),
            "msvc" => res.set_icon(icon_path),
            _ => panic!("Unsupported env: {}", target_env),
        };
        res.compile().unwrap();
    }
}

fn main() {
    add_icon_to_bin_when_building_for_win("./assets/actyxcli.ico");

    let arch = env::var("TARGET")
        .expect("TARGET not set")
        .split('-')
        .next()
        .expect("TARGET should have the right format")
        .to_string();

    // Since target_arch armv7 does not exist, we add our own cfg parameter
    println!("cargo:rustc-cfg=AX_ARCH=\"{}\"", arch);
}
