use std::str::FromStr;

use axossettings::{Scope, Validator};

#[test]
fn actyx_os_empty() {
    let schema = serde_json::from_reader(
        std::fs::File::open("../../../protocols/json-schema/os/node-settings.schema.json").unwrap(),
    )
    .unwrap();
    Validator::new(schema)
        .unwrap()
        .validate_with_defaults(None, &Scope::from_str(".").unwrap())
        .unwrap();
}
