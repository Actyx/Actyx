#![deny(clippy::future_not_send)]

mod database;
mod formats;
mod json_differ;
mod json_value;
mod repository;
mod scope;
mod validation;

pub use crate::settings::database::{Database, DB_FILENAME};
pub use crate::settings::repository::{Error as RepositoryError, Repository};
pub use crate::settings::scope::{Error as ScopeError, Scope};
pub use crate::settings::validation::{Error as ValidationError, ValidationErrorDescr, ValidationState, Validator};
