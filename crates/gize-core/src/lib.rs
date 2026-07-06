//! Core domain model and conventions for the Gize framework.
//!
//! This crate is intentionally framework-agnostic: it knows nothing about Axum, SQLx or
//! the CLI. It defines the vocabulary the rest of the workspace shares — the project
//! manifest (`gize.toml`), model/field definitions, and naming conventions.

pub mod field;
pub mod manifest;
pub mod naming;

pub use field::{Field, FieldType, ModelSpec, Relation};
pub use manifest::{Manifest, Module};
