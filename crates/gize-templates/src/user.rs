//! The built-in `users` resource that `gize new` scaffolds by default.
//!
//! It reuses the generic [`crate::crud`] templates for the vertical slice (dto, repository,
//! service, handler, routes, error, tests) and only customises the two files that a *users*
//! table wants to differ on:
//! - `model.rs`: `password` is never serialised into API responses (it holds a hash).
//! - the migration: `email` is `UNIQUE` and `is_admin` defaults to `false`.
//!
//! `is_admin` is included from day one so a future admin panel / `gize-auth` has a flag to
//! gate access on (see BACKLOG.md).

use gize_core::{Field, FieldType, ModelSpec};

/// The canonical `User` model `gize new` generates: the minimal, auth-ready field set.
///
/// `id`, `created_at` and `updated_at` are added by the templates, so they are not listed
/// here. This spec drives the reused CRUD templates (dto/repository/…).
pub fn spec() -> ModelSpec {
    let field = |name: &str, ty| Field {
        name: name.to_string(),
        ty,
    };
    ModelSpec {
        name: "User".to_string(),
        fields: vec![
            field("name", FieldType::String),
            field("email", FieldType::String),
            field("password", FieldType::String),
            field("is_admin", FieldType::Bool),
        ],
        relations: Vec::new(),
    }
}

/// `model.rs` for the users resource. Differs from the generic model only in that
/// `password` is skipped when serialising, so it never leaks into JSON responses.
pub fn model_rs() -> String {
    r#"use serde::{Deserialize, Serialize};

/// The `User` domain model, mapped to the `users` table.
///
/// `password` stores a hash and is never serialised into API responses. It is still read
/// from the database (`sqlx::FromRow`) and accepted on create/update through the DTOs.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
    // Loaded from the database but not serialised into responses; it is read once
    // authentication (login/password verification) lands. Allowed to be unused so a
    // freshly scaffolded app stays clippy-clean under `-D warnings`.
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    pub password: String,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
"#
    .to_string()
}

/// `CREATE TABLE users` migration. Adds a `UNIQUE` constraint on `email` and defaults
/// `is_admin` to `false` — both natural for a users table and not covered by the generic
/// model-driven migration (which keeps every column plain `NOT NULL`).
pub fn migration_sql() -> String {
    r#"-- Migration: create users
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_has_minimal_auth_fields() {
        let model = spec();
        assert_eq!(model.name, "User");
        let names: Vec<_> = model.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["name", "email", "password", "is_admin"]);
        assert_eq!(model.fields[3].ty, FieldType::Bool);
    }

    #[test]
    fn model_hides_password_from_responses() {
        let out = model_rs();
        assert!(out.contains("pub struct User"));
        assert!(out.contains("#[serde(skip_serializing)]\n    pub password: String,"));
        assert!(out.contains("#[allow(dead_code)]"));
        assert!(out.contains("pub is_admin: bool,"));
    }

    #[test]
    fn migration_creates_users_with_admin_flag() {
        let out = migration_sql();
        assert!(out.contains("CREATE TABLE users"));
        assert!(out.contains("email TEXT NOT NULL UNIQUE"));
        assert!(out.contains("is_admin BOOLEAN NOT NULL DEFAULT false"));
    }
}
