#![deny(clippy::future_not_send)]

#[macro_use]
extern crate log;

pub mod database;
pub mod json_differ;
pub mod json_value;
pub mod repository;
pub mod scope;
pub mod validation;

pub use database::Database;
pub use repository::Repository;
pub use scope::Scope;
pub use validation::Validator;
