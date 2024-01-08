use ax_core::DATABANK_VERSION;
use winres::WindowsResource;

pub fn add_icon_to_bin_when_building_for_win(icon_path: &str) {
    if std::env::var("CARGO_CFG_TARGET_FAMILY").unwrap() == "windows" {
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
    if !std::env::var("CARGO_PKG_VERSION")
        .unwrap()
        .starts_with(DATABANK_VERSION)
    {
        panic!("ax version MUST start the ax_core::DATABANK_VERSION");
    }

    add_icon_to_bin_when_building_for_win("./assets/actyxcli.ico");
}
