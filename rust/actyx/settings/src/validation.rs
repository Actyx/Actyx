use url::Url;
use valico::json_schema::{self, schema, validators};

#[derive(thiserror::Error, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Error {
    #[error("Provided schema is not valid. Error: {0}")]
    InvalidSchema(String),
    #[error("Validation failed.{0}")]
    ValidationFailed(ValidationState),
    #[error("Neither settings given nor default available for scope [{0}].")]
    MissingDefault(String),
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ValidationErrorDescr {
    pub path: String,
    pub title: String,
    pub detail: Option<String>,
}

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ValidationState {
    pub errors: Vec<ValidationErrorDescr>,
    pub missing: Vec<String>,
}

impl std::fmt::Display for ValidationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ValidationState { errors, missing } = self;
        let errors: Vec<String> = errors
            .iter()
            .map(|e| {
                let ValidationErrorDescr { path, title, detail } = e;
                let detail = detail
                    .to_owned()
                    .map(|d| format!(" ({}.)", d))
                    .unwrap_or_else(|| "".to_string());
                format!("\t\t{}: {}.{}", path, title, detail)
            })
            .collect();
        let errors = if errors.is_empty() {
            "".to_string()
        } else {
            format!("\n\tErrors:\n{}", errors.join("\n"))
        };
        let missing: Vec<String> = missing.iter().map(|m| format!("\t\t{}", m)).collect();
        let missing = if missing.is_empty() {
            "".to_string()
        } else {
            format!("\n\tMissing:\n{}", missing.join("\n"))
        };
        write!(f, "{}{}", errors, missing)
    }
}

impl From<validators::ValidationState> for ValidationState {
    fn from(s: validators::ValidationState) -> Self {
        let errors = s
            .errors
            .iter()
            .map(|e| {
                let title = e.get_title().to_string();
                let path = e.get_path().to_string();
                let detail = e.get_detail().map(|d| d.to_string());
                ValidationErrorDescr { path, title, detail }
            })
            .collect();
        let missing = s.missing.iter().map(|url| url.to_string()).collect();
        ValidationState { errors, missing }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Validator {
    schema: (Url, json_schema::Scope),
}
impl Validator {
    pub fn new(schema: serde_json::Value) -> Result<Self> {
        let mut scope = json_schema::Scope::with_formats(crate::formats::extra_formats).supply_defaults();
        let url = scope
            .compile(schema, false)
            .map_err(|err| Error::InvalidSchema(format!("{}", err)))?;

        Ok(Self { schema: (url, scope) })
    }

    fn get_schema(&'_ self) -> schema::ScopedSchema<'_> {
        let (url, scope) = &self.schema;
        scope.resolve(&url).unwrap()
    }

    fn handle_result<F>(
        mut validation_state: validators::ValidationState,
        default_if_no_replacement: F,
    ) -> Result<serde_json::Value>
    where
        F: FnOnce() -> serde_json::Value,
    {
        if validation_state.is_valid() {
            Ok(validation_state
                .replacement
                .take()
                .unwrap_or_else(default_if_no_replacement))
        } else {
            Err(Error::ValidationFailed(validation_state.into()))
        }
    }

    /// Validates a `json` value, given a `schema_json`. If individual fields are not set, but
    /// given a default in the schema, the default will be set. Note: If there are defaults given
    /// for multiple layers, the outer most one will be used.
    /// TODO: merge value with defaults?
    pub fn validate_with_defaults(
        &self,
        value: Option<&serde_json::Value>,
        scope: &crate::Scope,
    ) -> Result<serde_json::Value> {
        let schema = self.get_schema();
        if let Some(v) = value {
            Self::handle_result(schema.validate(v), || v.clone())
        } else {
            let defaults = schema
                .get_default()
                .ok_or_else(|| Error::MissingDefault(scope.to_string()))?;
            Self::handle_result(schema.validate(&defaults), || defaults)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::{json, Value};
    use std::fs::File;

    #[derive(Debug, PartialEq, serde::Deserialize)]
    struct Spec {
        schema: Value,
        input: Option<Value>,
        result: Result<Value>,
    }

    #[test]
    fn invalid_schema() {
        match Validator::new(serde_json::json!({ "not": "valid" })) {
            Err(Error::InvalidSchema(_)) => {}
            x => panic!("got {:?}", x),
        }
    }

    #[test]
    fn validation() {
        let test_suite: serde_json::map::Map<_, _> =
            serde_json::from_reader(std::fs::File::open("tests/validation.json").unwrap()).unwrap();

        for (name, spec) in test_suite.into_iter() {
            let Spec {
                schema,
                input,
                result: expected,
            } = serde_json::from_value(spec).unwrap();
            let validator = Validator::new(schema).unwrap();
            let result = validator.validate_with_defaults(input.as_ref(), &".".into());
            assert_eq!(result, expected, "spec: \"{}\"", name);
        }
    }

    #[test]
    fn should_work_with_extra_formats() {
        let schema_json: serde_json::Value =
            serde_json::from_reader(File::open("tests/schemas/multiaddr.schema.json").unwrap()).unwrap();
        let validator = Validator::new(schema_json).unwrap();

        let res = validator.validate_with_defaults(Some(&json!(1)), &".".into());
        if let Err(Error::ValidationFailed(err)) = res {
            assert_eq!(err.errors.len(), 2);
        } else {
            panic!("Expected ValidationFailed, got {:?}", res);
        }

        let res = validator.validate_with_defaults(Some(&json!("foo")), &".".into());
        if let Err(Error::ValidationFailed(err)) = res {
            assert_eq!(err.errors.len(), 1);
            assert_eq!(err.errors[0].title, "Format is wrong");
        } else {
            panic!("Expected ValidationFailed, got {:?}", res);
        }

        let res = validator.validate_with_defaults(
            Some(&json!(
                "/ip4/3.121.252.117/tcp/4001/p2p/QmaWM8pMoMYkJrdbUZkxHyUavH3tCxRdCC9NYCnXRfQ4Eg"
            )),
            &".".into(),
        );
        assert!(res.is_ok(), "got Err: {:?}", res.err());
    }
}
