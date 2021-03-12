use std::error::Error;
use util::build::add_icon_to_bin_when_building_for_win;

fn main() -> Result<(), Box<dyn Error>> {
    add_icon_to_bin_when_building_for_win("./assets/actyxcli.ico")
}
