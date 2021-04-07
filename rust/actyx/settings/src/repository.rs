use crate::{database, json_value::JsonValue, Scope, Validator};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error(transparent)]
    ValidationError(#[from] crate::validation::Error),
    #[error("Schema for scope '{0}' does not exist.")]
    SchemaNotFound(Scope),
    #[error("Scope '{0}' does not exist.")]
    ScopeNotFound(Scope),
    #[error("Couldn't handle JSON. Error: {0}")]
    JsonError(String),
    #[error("Database error: {0}")]
    DatabaseError(#[from] crate::database::Error),
    #[error(transparent)]
    UpdateError(#[from] crate::json_value::Error),
    #[error("No valid settings found under scope '{0}'.")]
    NoValidSettings(Scope),
    #[error("No settings found at scope '{0}'.")]
    NoSettingsAtScope(Scope),
    #[error("Root scope is not allowed.")]
    RootScopeNotAllowed,
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Repository {
    pub database: database::Database,
}

#[derive(Debug)]
pub struct SuccessfulValidation {
    pub schema_scope: Scope,
    pub object_with_defaults: serde_json::Value,
    pub object_without_defaults: serde_json::Value,
}

/// Validates a given object `obj` for a scope `scope`:
/// 1. Get settings scoped for found schema.
/// 2. Replace `obj` into scoped settings.
/// 3. Augment merged object with defaults from main schema.
/// 4. Validate this object according to the schema.
/// On success, the validated object and the schema's scope will be returned.
fn validate(
    schema_scope: &Scope,
    validator: &Validator,
    scope: &Scope,
    settings: serde_json::Value,
    global_settings: serde_json::Value,
) -> Result<SuccessfulValidation> {
    info!("Trying to validate {} with schema for {}", scope, schema_scope);
    let updated_schema_settings_without_defaults = if schema_scope == scope {
        settings
    } else {
        global_settings
            .update_at(scope, settings)?
            .pointer(schema_scope.as_json_ptr().as_str())
            .cloned()
            .unwrap() // we successfully did update_at() above so the pointer must be valid
    };
    debug!("Validating {}", updated_schema_settings_without_defaults);
    let res = validator
        .validate_with_defaults(Some(&updated_schema_settings_without_defaults), scope)
        .map(|object_with_defaults| SuccessfulValidation {
            schema_scope: schema_scope.clone(),
            object_with_defaults,
            object_without_defaults: updated_schema_settings_without_defaults,
        })?;
    Ok(res)
}

fn stringify(value: &serde_json::Value) -> Result<String> {
    serde_json::to_string(&value).map_err(|err| Error::JsonError(format!("{:?}", err)))
}

fn parse(s: String) -> Result<serde_json::Value> {
    serde_json::from_str(s.as_str()).map_err(|err| Error::JsonError(format!("{:?}", err)))
}

/// Searches recursively for a schema for `scope`, popping one level on each iteration.
/// Search terminates either if a schema is found or scope is empty.
fn parent_schema(tx: &mut database::Transaction, scope: &Scope) -> Result<(Scope, serde_json::Value)> {
    let res = parent_schema0(tx, &scope)?;
    res.ok_or_else(|| Error::SchemaNotFound(scope.clone()))
}
fn parent_schema0(tx: &mut database::Transaction, scope: &Scope) -> Result<Option<(Scope, serde_json::Value)>> {
    if scope.is_root() {
        Ok(None)
    } else {
        match tx.get_schema(scope.to_string())? {
            Some(schema) => {
                let schema = parse(schema)?;
                Ok(Some((scope.clone(), schema)))
            }
            None => parent_schema0(tx, &scope.drop_last()),
        }
    }
}

/// Creates a validator for the parent scope.
fn mk_validator(tx: &mut database::Transaction, scope: &Scope) -> Result<(Scope, Validator)> {
    let (schema_scope, schema) = parent_schema(tx, &scope)?;
    let res = Validator::new(schema)?;
    Ok((schema_scope, res))
}

impl Repository {
    pub fn new(database: database::Database) -> Result<Self> {
        Ok(Self { database })
    }

    pub fn new_in_memory() -> Self {
        Repository {
            database: database::Database::in_memory().unwrap(),
        }
    }

    /// Tries to replace the settings at given `scope` to `settings`. Unless `force` is set,
    /// providing a non-conformant (according to the installed schema) settings value will result
    /// in a rejection.
    /// If there is no schema installed for this scope, the validation will naturally fail (unless
    /// again, `force` is set).
    ///
    /// The `force` flag is provided to provide API users with bigger degree of freedom, as
    /// there is no enforced workflow when e.g. doing an incompatible schema update.
    ///
    /// If `force` is set then `settings` is returned as is. Otherwise the new settings of the parent
    /// schema are returned.
    pub fn update_settings(
        &mut self,
        scope: &Scope,
        settings: serde_json::Value,
        force: bool,
    ) -> Result<serde_json::Value> {
        self.database.exec(|tx| {
            let current_settings = tx
                .get_settings()?
                .map(parse)
                .transpose()?
                .unwrap_or_else(|| serde_json::json!({}));

            let (schema_scope, validator) = mk_validator(tx, &scope)?;

            let validation = validate(
                &schema_scope,
                &validator,
                &scope,
                settings.clone(),
                current_settings.clone(),
            );
            match validation {
                Ok(SuccessfulValidation {
                    schema_scope,
                    object_with_defaults: new_settings_with_defaults,
                    object_without_defaults: new_settings_without_defaults,
                }) => {
                    debug!(
                        "Successful validation, new_settings_with_defaults: {}",
                        new_settings_with_defaults
                    );
                    let new_settings = current_settings.update_at(&schema_scope, new_settings_without_defaults)?;
                    tx.set_settings(stringify(&new_settings)?)?;
                    Ok(new_settings_with_defaults)
                }
                Err(Error::ValidationError(err)) if force => {
                    let new_settings = current_settings.update_at_force(&scope, settings.clone());
                    info!(
                        "Validation failed with error {}. Force is enabled so {} will be set to {}.",
                        err, scope, new_settings
                    );
                    tx.set_settings(stringify(&new_settings)?)?;
                    Ok(settings)
                }
                Err(e) => Err(e), // unrecoverable
            }
        })?
    }

    // Clears settings for a given scope,
    // if the defaults are valid on their own, the settings_with_defaults will still be set
    pub fn clear_settings(&mut self, scope: &Scope) -> Result<()> {
        self.database.exec(|tx| {
            if let Some(current_settings) = tx.get_settings()?.map(parse).transpose()? {
                let new_settings = current_settings.remove_at(&scope);
                tx.set_settings(stringify(&new_settings)?)?;
            }
            Ok(())
        })?
    }

    fn get_schema_settings(
        tx: &mut database::Transaction,
        current_settings: Option<&serde_json::Value>,
        scope: &Scope,
        no_defaults: bool,
    ) -> Result<Option<(Scope, serde_json::Value)>> {
        let (schema_scope, validator) = mk_validator(tx, scope)?;
        let schema_settings = current_settings.and_then(|c| c.pointer(&schema_scope.as_json_ptr()).cloned());
        let res = if no_defaults {
            schema_settings
        } else {
            Some(
                validator
                    .validate_with_defaults(schema_settings.as_ref(), scope)
                    .map_err(|_| Error::NoValidSettings(scope.clone()))?,
            )
        };
        Ok(res.map(|settings| (schema_scope, settings)))
    }

    /// Returns settings for a given scope. If the `no_defaults` flag is set, settings will be
    /// unconditionally returned, even if they might be non-conformant to the installed schema, or
    /// there is no schema available for the same scope.
    /// If `no_defaults` is set to false, the returned object is guaranteed to be valid (if there's a
    /// schema installed).
    /// If the provided scope is the root scope, the settings object will be returned without any
    /// validation, irrespective of the `no_defaults` flag.
    pub fn get_settings(&mut self, scope: &Scope, no_defaults: bool) -> Result<serde_json::Value> {
        self.database.exec(|tx| {
            let current_settings = tx.get_settings()?.map(parse).transpose()?;
            if scope.is_root() {
                if no_defaults {
                    current_settings.ok_or_else(|| Error::NoSettingsAtScope(scope.clone()))
                } else {
                    let mut scopes = tx
                        .get_all_schema_scopes()?
                        .into_iter()
                        .map(|s| <Scope as std::convert::TryFrom<String>>::try_from(s).unwrap())
                        .collect::<Vec<Scope>>();
                    scopes.sort_by_key(|scope| scope.iter().len());
                    let all_settings_with_defaults = scopes
                        .into_iter()
                        .filter_map(|scope| {
                            Self::get_schema_settings(tx, current_settings.as_ref(), &scope, false).transpose()
                        })
                        .collect::<Result<Vec<(Scope, serde_json::Value)>>>()?
                        .into_iter()
                        .try_fold(serde_json::json!({}), |acc, (scope, settings)| {
                            acc.update_at(&scope, settings)
                        })?;
                    Ok(all_settings_with_defaults)
                }
            } else {
                let scope_and_settings = Self::get_schema_settings(tx, current_settings.as_ref(), &scope, no_defaults)?;
                scope_and_settings
                    .and_then(|(schema_scope, settings)| match scope.diff(&schema_scope) {
                        Some(scope_within_schema) => settings.pointer(&scope_within_schema.as_json_ptr()).cloned(),
                        None => Some(settings),
                    })
                    .ok_or_else(|| Error::NoSettingsAtScope(scope.clone()))
            }
        })?
    }

    /// Deletes a schema for a given `scope`. This will also delete any settings stored for the
    /// same scope in one atomic operation.
    pub fn delete_schema(&mut self, scope: &Scope) -> Result<()> {
        self.database.exec(|tx| {
            if !tx.delete_schema(scope.into())? {
                return Err(Error::SchemaNotFound(scope.clone()));
            }

            if let Some(current_settings) = tx.get_settings()?.map(parse).transpose()? {
                let new_settings = current_settings.remove_at(&scope);
                tx.set_settings(stringify(&new_settings)?)?;
            }
            Ok(())
        })?
    }

    /// Sets a schema for a given scope. If the provided `schema` is a valid JSON schema, this will
    /// unconditionally replace an existing schema for the same scope. Meaning, any installed
    /// settings for that scope, that were valid according to the old schema, could now become
    /// invalid.
    ///
    /// Note: It's not supported to set a schema for the root scope.
    pub fn set_schema(&mut self, scope: &Scope, schema: serde_json::Value) -> Result<()> {
        if scope.is_root() {
            return Err(Error::RootScopeNotAllowed);
        }

        self.database.exec(|tx| {
            tx.set_schema(scope.into(), stringify(&schema)?)?;
            Ok(())
        })?
    }

    /// Returns an installed schema for a given `scope`, if any.
    pub fn get_schema(&mut self, scope: &Scope) -> Result<serde_json::Value> {
        self.database.exec(|tx| {
            let schema = tx
                .get_schema(scope.into())?
                .map(parse)
                .transpose()?
                .ok_or_else(|| Error::SchemaNotFound(scope.clone()))?;
            Ok(schema)
        })?
    }

    /// Returns all installed schemas with their respective scopes.
    pub fn get_schema_scopes(&mut self) -> Result<Vec<String>> {
        self.database.exec(|tx| {
            let schemas = tx.get_all_schema_scopes()?;
            Ok(schemas)
        })?
    }
}
