#![deny(clippy::future_not_send)]

#[macro_use]
extern crate log;

mod database;
mod formats;
mod json_differ;
mod json_value;
mod repository;
mod scope;
mod validation;

pub use crate::database::Database;
pub use crate::repository::{Error as RepositoryError, Repository};
pub use crate::scope::{Error as ScopeError, Scope};
pub use crate::validation::{Error as ValidationError, ValidationErrorDescr, ValidationState, Validator};
