//! Templates for the code Gize generates.
//!
//! For the MVP skeleton these are Rust functions returning file contents. ADR-004 plans a
//! migration to `minijinja` templates loaded from disk; the function boundary here is
//! designed so that swap is internal and does not change the generator's API.

pub mod crud;
pub mod model;
pub mod module;
pub mod project;
pub mod user;

pub use model::{blank_migration_sql, migration_sql, model_rs};
