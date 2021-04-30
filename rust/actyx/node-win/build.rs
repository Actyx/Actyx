use util::build::add_icon_to_bin_when_building_for_win;

fn main() {
    // This only works if there's a single binary target for the crate. As
    // linking the resource file statically won't do anything for a lib target.
    // So far, there's no way to do that using Cargo.
    add_icon_to_bin_when_building_for_win("./assets/actyx.ico")
}
