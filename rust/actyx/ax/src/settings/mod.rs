#![deny(clippy::future_not_send)]

mod database;
mod formats;
mod json_differ;
mod json_value;
mod repository;
mod scope;
mod validation;

pub use crate::settings::{
    database::{Database, DB_FILENAME},
    repository::{Error as RepositoryError, Repository},
    scope::{Error as ScopeError, Scope},
    validation::{Error as ValidationError, ValidationErrorDescr, ValidationState, Validator},
};
