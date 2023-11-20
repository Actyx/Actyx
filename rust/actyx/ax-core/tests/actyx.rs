use ax_core::settings::{Scope, Validator};
use std::str::FromStr;

#[test]
fn defaults() {
    let schema =
        serde_json::from_reader(std::fs::File::open("resources/json-schema/node-settings.schema.json").unwrap())
            .unwrap();
    Validator::new(schema)
        .unwrap()
        .validate_with_defaults(None, &Scope::from_str(".").unwrap())
        .unwrap();
}
