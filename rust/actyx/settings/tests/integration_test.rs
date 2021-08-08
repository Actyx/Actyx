use serde_json::json;
use settings::{
    Database, Repository, RepositoryError as Error, Scope, ValidationError, ValidationErrorDescr, ValidationState,
};
use std::{fs, path::PathBuf, str::FromStr};
use tempfile::{tempdir, TempDir};

fn load_schema(path: PathBuf) -> serde_json::Value {
    serde_json::from_reader(fs::File::open(path).unwrap()).unwrap()
}

fn repo(path: &TempDir) -> Repository {
    Repository::new(Database::new(path.path().to_path_buf()).unwrap())
}

// Test cases
// 1) no scope -> get config -> empty config
// 2) set schema -> set config -> get config with defaults
// 3) set schema -> set invalid config -> assert error
// 4) (2), then update with incompatible schema -> assert error -> set invalid config, ignoring
//    errors -> assert -> set valid config -> assert
// 5) (2) then set 2nd schema with full defaults -> get config with all defaults -> get config with no defaults
// 6) set config for root scope
// 7) set schema twice (whose defaults don't yield a valid config)
// 8) smoke test for current ActyxOS schema
// 9) set schema for root scope
// 10) schema allowing everything
// 11) test unsetting and fallback to defaults

// Make tests chainable
type TestResult = std::result::Result<TempDir, Error>;
fn testcase_1(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let root_settings = repo.get_settings(&Scope::root(), false)?;
    assert_eq!(root_settings, json!({}));
    let scope = Scope::from_str("com.actyx/weird").unwrap();
    assert_eq!(
        repo.get_settings(&scope, false),
        Err(Error::SchemaNotFound(scope.clone())),
    );
    Ok(dir)
}

fn testcase_2(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test1.schema.json".into());
    let scope: Scope = Scope::from_str("com.example.testcase_2").unwrap();
    match repo.delete_schema(&scope) {
        Ok(()) => { /* depends on test order */ }
        Err(Error::SchemaNotFound(err_scope)) => assert_eq!(scope, err_scope),
        _ => panic!(),
    }
    repo.set_schema(&scope, schema)?;
    let default_config = repo.get_settings(&scope, false)?;
    assert_eq!(default_config, json!({"backgroundColor":"green"}));
    assert_eq!(
        repo.get_settings(&scope, true),
        Err(Error::NoSettingsAtScope(scope.clone())),
    );

    let new_config = json!({"backgroundColor":"orange"});
    let updated = repo.update_settings(&scope, new_config, false)?;
    assert_eq!(updated, json!({"backgroundColor":"orange"}));

    let query_updated = repo.get_settings(&scope, false)?;
    assert_eq!(query_updated, json!({"backgroundColor":"orange"}));
    Ok(dir)
}

fn testcase_3(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test1.schema.json".into());
    let scope = Scope::from_str("com.example.sample").unwrap();
    repo.set_schema(&scope, schema)?;
    let invalid_config = json!({ "backgroundColor": 42 });
    assert_eq!(
        repo.update_settings(&scope, invalid_config, false),
        Err(Error::ValidationError(ValidationError::ValidationFailed(
            ValidationState {
                errors: vec![ValidationErrorDescr {
                    path: "/backgroundColor".to_string(),
                    title: "Type of the value is wrong".to_string(),
                    detail: Some("The value must be string".to_string())
                }],
                missing: vec![]
            }
        )))
    );

    Ok(dir)
}

fn testcase_4(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test1.schema.json".into());
    let scope = Scope::from_str("com.example.testcase_4").unwrap();
    repo.set_schema(&scope, schema)?;
    let default_config = repo.get_settings(&scope, false)?;
    assert_eq!(default_config, json!({ "backgroundColor": "green" }));
    assert_eq!(
        repo.get_settings(&scope, true),
        Err(Error::NoSettingsAtScope(scope.clone())),
    );

    let new_config = json!({ "backgroundColor": "orange" });
    let updated = repo.update_settings(&scope, new_config.clone(), false)?;
    assert_eq!(updated, new_config);
    let query_updated = repo.get_settings(&scope, false)?;
    assert_eq!(query_updated, new_config);
    let incompatible_schema = load_schema("tests/schemas/test4.schema.json".into());
    repo.set_schema(&scope, incompatible_schema)?;
    assert_eq!(
        repo.get_settings(&scope, false),
        Err(Error::NoValidSettings(scope.clone())),
    );
    assert_eq!(repo.get_settings(&scope, true)?, json!({ "backgroundColor": "orange" }));

    let wrong_config = json!({ "backgroundColor": "wrongAgain" });
    assert_eq!(repo.update_settings(&scope, wrong_config.clone(), true)?, wrong_config);
    assert_eq!(repo.get_settings(&scope, true)?, wrong_config);
    assert_eq!(
        repo.get_settings(&scope, false),
        Err(Error::NoValidSettings(scope.clone())),
    );

    let correct_config = json!({"backgroundColor":31337});
    assert_eq!(
        repo.update_settings(&scope, correct_config.clone(), true)?,
        correct_config
    );
    assert_eq!(repo.get_settings(&scope, true)?, correct_config);
    assert_eq!(repo.get_settings(&scope, false)?, correct_config);

    Ok(dir)
}

fn testcase_5(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let dir = testcase_2(dir)?;
    let second_schema = load_schema("tests/schemas/test2.schema.json".into());
    let scope = Scope::from_str("com.actyx").unwrap();
    repo.set_schema(&scope, second_schema)?;

    assert_eq!(
        repo.get_settings(&scope, false),
        Err(Error::NoValidSettings(scope.clone())),
    );
    assert_eq!(
        repo.get_settings(&scope, true),
        Err(Error::NoSettingsAtScope(scope.clone())),
    );

    let update_response = repo.update_settings(
        &scope,
        json!({"general":{"someVal":"f44f72cab04e062d86bfc7afb04bd6f7d73a48a11b8584a49e8c0e9e2b4822d9","someObject":{"ip":"someIp","port":"somePort","pubKey":"someKey"}}}),
        false
    )?;
    assert_eq!(
        update_response,
        json!({"general":{"someObject":{"ip":"someIp","port":"somePort","pubKey":"someKey"},"name":"Random Node","someVal":"f44f72cab04e062d86bfc7afb04bd6f7d73a48a11b8584a49e8c0e9e2b4822d9"}})
    );

    let config_with_defaults = repo.get_settings(&scope, false)?;
    assert_eq!(
        config_with_defaults,
        json!({"general":{"someObject":{"ip":"someIp","port":"somePort","pubKey":"someKey"},"name":"Random Node","someVal":"f44f72cab04e062d86bfc7afb04bd6f7d73a48a11b8584a49e8c0e9e2b4822d9"}})
    );
    let config_without_defaults = repo.get_settings(&scope, true)?;
    // Note the missing `name`, which is a required property with a default.
    assert_eq!(
        config_without_defaults,
        json!({"general":{"someObject":{"ip":"someIp","port":"somePort","pubKey":"someKey"},"someVal":"f44f72cab04e062d86bfc7afb04bd6f7d73a48a11b8584a49e8c0e9e2b4822d9"}})
    );
    Ok(dir)
}

fn testcase_6(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test1.schema.json".into());
    let scope = Scope::from_str("com.sample.test6").unwrap();
    repo.set_schema(&scope, schema)?;
    let default_config = repo.get_settings(&scope, false).unwrap();
    assert_eq!(default_config, json!({ "backgroundColor": "green" }));
    assert_eq!(
        repo.get_settings(&scope, true),
        Err(Error::NoSettingsAtScope(scope.clone())),
    );

    let new_config = json!({"backgroundColor":"orange"});
    let updated = repo.update_settings(&scope, new_config, false)?;
    assert_eq!(updated, json!({"backgroundColor":"orange"}));

    let query_updated = repo.get_settings(&scope, false)?;
    assert_eq!(query_updated, json!({"backgroundColor":"orange"}));
    Ok(dir)
}

fn testcase_7(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test2.schema.json".into());
    let scope = Scope::from_str("test7").unwrap();
    repo.set_schema(&scope, schema.clone())?;

    repo.set_schema(&scope, schema)?;
    Ok(dir)
}

fn testcase_8(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test6.schema.json".into());
    let scope = Scope::from_str("com.actyx8").unwrap();
    repo.set_schema(&scope, schema)?;

    let set_config = repo.update_settings(
        &scope,
        json!({
          "general": {
            "someVal": "f44f72cab04e062d86bfc7afb04bd6f7d73a48a11b8584a49e8c0e9e2b4822d9",
            "someObjects": ["/ip4/18.184.146.163/tcp/4001/ipfs/QmcTQtFCTtdv8y3PFzK5zWydBgARxwXfurUaoEkjqa7pS8"],
            "name": "Dev"
          },
          "services": {
            "eventService": {
              "readOnly": false,
              "topic": "hot"
            }
          }
        }),
        false,
    )?;
    assert_eq!(
        set_config,
        json!({"general":{"someObjects":["/ip4/18.184.146.163/tcp/4001/ipfs/QmcTQtFCTtdv8y3PFzK5zWydBgARxwXfurUaoEkjqa7pS8"],"name":"Dev","logLevels":{"apps":"INFO","os":"INFO"},"someVal":"f44f72cab04e062d86bfc7afb04bd6f7d73a48a11b8584a49e8c0e9e2b4822d9"},"services":{"dockerRuntime":{"appRestartPolicy":"unless-stopped"},"eventService":{"readOnly":false,"topic":"hot"}}})
    );

    let update_scope = scope.append(&Scope::from_str("general/name").unwrap());
    let update_single_field = repo.update_settings(&update_scope, json!("Some other name"), false)?;
    assert_eq!(update_single_field, json!("Some other name"));

    let get_config = repo.get_settings(&scope, false)?;
    assert_eq!(
        get_config,
        json!({"general":{"someObjects":["/ip4/18.184.146.163/tcp/4001/ipfs/QmcTQtFCTtdv8y3PFzK5zWydBgARxwXfurUaoEkjqa7pS8"],"name":"Some other name","logLevels":{"apps":"INFO","os":"INFO"},"someVal":"f44f72cab04e062d86bfc7afb04bd6f7d73a48a11b8584a49e8c0e9e2b4822d9"},"services":{"dockerRuntime":{"appRestartPolicy":"unless-stopped"},"eventService":{"readOnly":false,"topic":"hot"}}})
    );
    Ok(dir)
}

#[allow(clippy::unnecessary_wraps)]
fn testcase_9(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test1.schema.json".into());
    assert_eq!(repo.set_schema(&Scope::root(), schema), Err(Error::RootScopeNotAllowed));
    Ok(dir)
}

#[allow(clippy::unnecessary_wraps)]
fn testcase_10(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test7.schema.json".into());
    let scope = Scope::from_str("com.actyx.test7").unwrap();
    repo.set_schema(&scope, schema).unwrap();
    assert_eq!(
        repo.get_settings(&scope, true),
        Err(Error::NoSettingsAtScope(scope.clone())),
    );
    Ok(dir)
}

fn testcase_11(dir: TempDir) -> TestResult {
    let dir = testcase_8(dir)?;
    let repo = repo(&dir);
    let scope = Scope::from_str("com.actyx8/general/logLevels/os").unwrap();
    repo.clear_settings(&scope).unwrap();
    let log_level = repo.get_settings(&scope, false).unwrap();
    assert_eq!(log_level.as_str().unwrap(), "INFO");

    repo.update_settings(&scope, json!("WARN"), false).unwrap();
    let log_level = repo.get_settings(&scope, false).unwrap();
    assert_eq!(log_level.as_str().unwrap(), "WARN");

    repo.clear_settings(&scope).unwrap();
    let log_level = repo.get_settings(&scope, false).unwrap();
    assert_eq!(log_level.as_str().unwrap(), "INFO");

    Ok(dir)
}

fn testcase_12(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test8.schema.json".into());
    let scope = Scope::from_str("com.actyx.test12").unwrap();
    repo.set_schema(&scope, schema)?;

    let default_config = repo.get_settings(&scope, false).unwrap();
    assert_eq!(
        default_config,
        json!({ "parent": { "child1": "<placeholder>", "child2": {} } })
    );

    let scope_child = scope.append(&Scope::from_str("parent/child1").unwrap());
    repo.update_settings(&scope_child, json!(42), false).unwrap();
    let child1 = repo.get_settings(&scope_child, false).unwrap();
    assert_eq!(child1.as_i64().unwrap(), 42);

    repo.clear_settings(&scope_child).unwrap();
    let child1 = repo.get_settings(&scope_child, false).unwrap();
    assert_eq!(child1.as_str().unwrap(), "<placeholder>");

    Ok(dir)
}

fn testcase_13(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test9.schema.json".into());
    let scope = Scope::from_str("com.actyx.test13").unwrap();
    repo.set_schema(&scope, schema)?;

    let schema_settings_without_defaults = repo.get_settings(&scope, true);
    assert_eq!(
        schema_settings_without_defaults,
        Err(Error::NoSettingsAtScope(scope.clone()))
    );

    let schema_settings = repo.get_settings(&scope, false).unwrap();
    assert_eq!(schema_settings, json!({ "parent": { "child": "<placeholder>" } }));

    let scope_child = scope.append(&Scope::from_str("parent/child").unwrap());
    repo.update_settings(&scope_child, json!("replacement"), false).unwrap();
    let child = repo.get_settings(&scope_child, false).unwrap();
    assert_eq!(child.as_str().unwrap(), "replacement");

    repo.clear_settings(&scope_child).unwrap();
    let child = repo.get_settings(&scope_child, false).unwrap();
    assert_eq!(child.as_str().unwrap(), "<placeholder>");

    let schema_settings_without_defaults = repo.get_settings(&scope, true).unwrap();
    assert_eq!(schema_settings_without_defaults, json!({ "parent": {} }));

    Ok(dir)
}

fn testcase_14(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test9.schema.json".into());
    let scope = Scope::from_str("com.actyx.test14").unwrap();
    repo.set_schema(&scope, schema)?;

    let non_existant_scope = scope.append(&"parent/child/non-existant".parse().unwrap());
    assert_eq!(
        repo.get_settings(&non_existant_scope, false),
        Err(Error::NoSettingsAtScope(non_existant_scope)),
    );

    Ok(dir)
}

fn testcase_15(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test8.schema.json".into());
    let scope = Scope::from_str("com.actyx.test12").unwrap();
    repo.set_schema(&scope, schema)?;

    let settings = repo.update_settings(&Scope::root(), json!(42), false);
    assert_eq!(settings, Err(Error::SchemaNotFound(Scope::root())));

    Ok(dir)
}

fn testcase_16(dir: TempDir) -> TestResult {
    let repo = repo(&dir);
    let schema = load_schema("tests/schemas/test10.schema.json".into());
    let scope = Scope::from_str("com.actyx.test16").unwrap();
    repo.set_schema(&scope, schema)?;

    let settings = repo
        .update_settings(&scope.append(&"0".parse().unwrap()), json!(42), false)
        .unwrap();
    assert_eq!(settings, json!(42));

    let settings = repo.update_settings(&scope, json!([42]), false).unwrap();
    assert_eq!(settings, json!([42]));

    Ok(dir)
}

fn testcase_17(dir: TempDir) -> TestResult {
    let repo = repo(&dir);

    let with_defaults_scope = Scope::from_str("com.actyx.test101").unwrap();
    let with_defaults_schema = load_schema("tests/schemas/test9.schema.json".into());
    repo.set_schema(&with_defaults_scope, with_defaults_schema)?;

    let without_defaults_scope = Scope::from_str("com.actyx.test102").unwrap();
    let without_defaults_schema = load_schema("tests/schemas/test10.schema.json".into());
    repo.set_schema(&without_defaults_scope, without_defaults_schema)?;

    assert_eq!(
        repo.get_settings(&Scope::root(), true),
        Err(Error::NoSettingsAtScope(Scope::root())),
    );
    assert_eq!(
        repo.get_settings(&Scope::root(), false),
        Err(Error::NoValidSettings(without_defaults_scope.clone())),
    );
    assert_eq!(
        repo.update_settings(&without_defaults_scope, serde_json::json!([1, 2, 3]), false)
            .unwrap(),
        serde_json::json!([1, 2, 3]),
    );
    assert_eq!(
        repo.update_settings(&without_defaults_scope, serde_json::json!([1, 2, 3]), true)
            .unwrap(),
        serde_json::json!([1, 2, 3]),
    );
    assert_eq!(
        repo.get_settings(&Scope::root(), false)?,
        serde_json::json!({
          "com.actyx.test101": {"parent": {"child": "<placeholder>"}},
          "com.actyx.test102": [1, 2, 3]
        }),
    );
    Ok(dir)
}

fn testcase_18(dir: TempDir) -> TestResult {
    let repo = repo(&dir);

    let scope = Scope::from_str("com.actyx.test101").unwrap();
    let schema = load_schema("tests/schemas/test9.schema.json".into());
    repo.set_schema(&scope, schema)?;

    let parent_scope = scope.append(&"parent".parse().unwrap());

    assert_eq!(
        repo.get_settings(&scope, false).unwrap(),
        serde_json::json!({"parent": {"child": "<placeholder>"} }),
    );
    assert_eq!(
        repo.get_settings(&scope, true),
        Err(Error::NoSettingsAtScope(scope.clone())),
    );
    assert_eq!(
        repo.update_settings(&parent_scope, serde_json::json!({ "child": "temp" }), false)
            .unwrap(),
        serde_json::json!({ "child": "temp" }),
    );
    assert_eq!(
        repo.update_settings(&parent_scope, serde_json::json!("nope"), true)
            .unwrap(),
        serde_json::json!("nope"),
    );
    assert_eq!(
        repo.get_settings(&parent_scope, false),
        Err(Error::NoValidSettings(parent_scope.clone())),
    );
    assert_eq!(
        repo.get_settings(&parent_scope, true).unwrap(),
        serde_json::json!("nope"),
    );
    repo.clear_settings(&parent_scope).unwrap();
    assert_eq!(
        repo.get_settings(&scope, false).unwrap(),
        serde_json::json!({"parent": {"child": "<placeholder>"} }),
    );
    Ok(dir)
}

#[test]
fn test_1() {
    let dir = tempdir().unwrap();
    testcase_1(dir).unwrap();
}

#[test]
fn test_6() {
    let dir = tempdir().unwrap();
    testcase_6(dir).unwrap();
}
#[test]
fn test_11() {
    let dir = tempdir().unwrap();
    testcase_11(dir).unwrap();
}
#[test]
fn test_12() {
    let dir = tempdir().unwrap();
    testcase_12(dir).unwrap();
}
#[test]
fn test_13() {
    let dir = tempdir().unwrap();
    testcase_13(dir).unwrap();
}
#[test]
fn test_14() {
    let dir = tempdir().unwrap();
    testcase_14(dir).unwrap();
}
#[test]
fn test_15() {
    let dir = tempdir().unwrap();
    testcase_15(dir).unwrap();
}
#[test]
fn test_16() {
    let dir = tempdir().unwrap();
    testcase_16(dir).unwrap();
}

#[test]
fn root_scope_defaults() {
    let dir = tempdir().unwrap();
    testcase_17(dir).unwrap();
}

#[test]
fn invalid() {
    let dir = tempdir().unwrap();
    testcase_18(dir).unwrap();
}

macro_rules! shuffle{
    ($($fn_name:ident),*) => {
        {
            use rand::seq::SliceRandom;
            let mut vec: Vec<(String, Box<dyn Fn(TempDir) -> TestResult>)> = vec![
            $((stringify!($fn_name).to_string(), Box::new($fn_name))),*
            ];
            vec.shuffle(&mut rand::thread_rng());
            vec
        }
    }
}

#[test]
fn shuffle() {
    for _ in 0..10 {
        // testcase_{1,6} is not independent
        let tests = shuffle![
            testcase_2,
            testcase_3,
            testcase_4,
            testcase_5,
            testcase_7,
            testcase_8,
            testcase_9,
            testcase_10
        ];
        println!(
            "Sequence of tests: {:?}",
            tests.iter().map(|(name, _)| name).collect::<Vec<_>>()
        );

        tests
            .into_iter()
            .map(|(_, f)| f)
            .fold(tempdir().unwrap(), |dir, fun| fun(dir).unwrap());
    }
}
