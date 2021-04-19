fn main() {
    #[cfg(windows)]
    util::build::add_icon_to_bin_when_building_for_win("./assets/actyxcli.ico");
}
