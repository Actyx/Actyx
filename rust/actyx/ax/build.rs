use util::build::add_icon_to_bin_when_building_for_win;

fn main() {
    add_icon_to_bin_when_building_for_win("./assets/actyxcli.ico");

    let version = util::version::NodeVersion::get_cli();
    println!("cargo:rustc-env=AX_CLI_VERSION={}", version);
    println!("cargo:rerun-if-env-changed=ACTYX_VERSION_CLI");
}
