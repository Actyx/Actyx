use std::str::FromStr;

use settings::{Scope, Validator};

#[test]
fn defaults() {
    let schema = serde_json::from_reader(
        std::fs::File::open("../../../protocols/json-schema/node-settings.schema.json").unwrap(),
    )
    .unwrap();
    Validator::new(schema)
        .unwrap()
        .validate_with_defaults(None, &Scope::from_str(".").unwrap())
        .unwrap();
}
