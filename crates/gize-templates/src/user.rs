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

use gize_core::{Dialect, Field, FieldType, ModelSpec};

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

/// `dto.rs` for users: the generic Create/Update payloads plus a `LoginRequest` and the
/// `TokenResponse` returned by register/login.
pub fn dto_rs() -> String {
    r#"use serde::{Deserialize, Serialize};
use validator::Validate;

/// Payload to create a `User`. `password` is plaintext on the wire and hashed before storage.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUser {
    #[validate(length(min = 1, message = "must not be empty"))]
    pub name: String,
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 8, message = "must be at least 8 characters"))]
    pub password: String,
    pub is_admin: bool,
}

/// Payload to update a `User`.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUser {
    #[validate(length(min = 1, message = "must not be empty"))]
    pub name: String,
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 8, message = "must be at least 8 characters"))]
    pub password: String,
    pub is_admin: bool,
}

/// Credentials for `POST /users/login`.
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 1, message = "must not be empty"))]
    pub password: String,
}

/// A signed session token returned by register/login.
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub token: String,
}
"#
    .to_string()
}

/// `error.rs` for users: the generic errors plus `Unauthorized` (bad credentials) and
/// `Internal` (hashing/token failures), so the auth handlers map cleanly to HTTP.
pub fn error_rs(dialect: Dialect) -> String {
    r#"use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Errors returned by the `users` resource.
#[derive(Debug)]
pub enum Error {
    NotFound,
    Unauthorized,
    /// A unique constraint was violated (e.g. a duplicate email) — maps to 409.
    Conflict,
    /// A foreign-key constraint was violated (a referenced record is missing, or is still
    /// referenced by another row) — maps to 409.
    ForeignKey,
    /// Request payload failed validation — maps to 422.
    Validation(String),
    Internal,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for Error {
    fn from(error: sqlx::Error) -> Self {
        if let sqlx::Error::RowNotFound = error {
            return Error::NotFound;
        }
        // Map database integrity violations to client errors instead of a generic 500.
        if let sqlx::Error::Database(ref db) = error {
            match db.code().as_deref() {
                Some("__UNIQUE_CODE__") => return Error::Conflict,   // unique violation
                Some("__FK_CODE__") => return Error::ForeignKey, // foreign-key violation
                _ => {}
            }
        }
        Error::Database(error)
    }
}

impl From<validator::ValidationErrors> for Error {
    fn from(errors: validator::ValidationErrors) -> Self {
        Error::Validation(errors.to_string())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "invalid credentials".to_string()),
            Error::Conflict => (StatusCode::CONFLICT, "already exists".to_string()),
            Error::ForeignKey => (
                StatusCode::CONFLICT,
                "a referenced record does not exist or is still in use".to_string(),
            ),
            Error::Validation(message) => (StatusCode::UNPROCESSABLE_ENTITY, message),
            Error::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_string()),
            Error::Database(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
        };
        (status, message).into_response()
    }
}
"#
    .replace("__UNIQUE_CODE__", dialect.unique_violation_code())
    .replace("__FK_CODE__", dialect.foreign_key_violation_code())
}

/// `handler.rs` for users: the CRUD handlers (hashing the password on create/update) plus
/// public `register` and `login` handlers that issue a token (ADR-013).
pub fn handler_rs(dialect: Dialect) -> String {
    r#"use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use validator::Validate;

use super::dto::{CreateUser, LoginRequest, TokenResponse, UpdateUser};
use super::error::Error;
use super::model::User;
use super::service;
use crate::auth::{hash_password, issue_token, verify_password};
use crate::state::AppState;

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<User>>, Error> {
    Ok(Json(service::list(&state.db).await?))
}

pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<User>, Error> {
    Ok(Json(service::find(&state.db, id).await?))
}

pub async fn create(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), Error> {
    input.validate()?;
    let stored = CreateUser {
        password: hash_password(&input.password).map_err(|_| Error::Internal)?,
        ..input
    };
    let user = service::create(&state.db, &stored).await?;
    Ok((StatusCode::CREATED, Json(user)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(input): Json<UpdateUser>,
) -> Result<Json<User>, Error> {
    input.validate()?;
    let stored = UpdateUser {
        password: hash_password(&input.password).map_err(|_| Error::Internal)?,
        ..input
    };
    Ok(Json(service::update(&state.db, id, &stored).await?))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<StatusCode, Error> {
    service::delete(&state.db, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Public: register a new user (hashing the password) and return a session token.
///
/// `is_admin` is forced to `false` here — this is a public endpoint, so a client must not be
/// able to grant itself admin. Admins are created through the guarded `POST /users` route.
pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<(StatusCode, Json<TokenResponse>), Error> {
    input.validate()?;
    let stored = CreateUser {
        password: hash_password(&input.password).map_err(|_| Error::Internal)?,
        is_admin: false,
        ..input
    };
    let user = service::create(&state.db, &stored).await?;
    let token = issue_token(&user.id).map_err(|_| Error::Internal)?;
    Ok((StatusCode::CREATED, Json(TokenResponse { token })))
}

/// Public: exchange email + password for a session token.
pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<TokenResponse>, Error> {
    input.validate()?;
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = __P1__")
        .bind(&input.email)
        .fetch_optional(&state.db)
        .await?
        .ok_or(Error::Unauthorized)?;
    if !verify_password(&input.password, &user.password) {
        return Err(Error::Unauthorized);
    }
    let token = issue_token(&user.id).map_err(|_| Error::Internal)?;
    Ok(Json(TokenResponse { token }))
}
"#
    .replace("__P1__", &dialect.placeholder(1))
}

/// `routes.rs` for users: public register/login and reads, with the write routes guarded by
/// the auth middleware (ADR-013).
pub fn routes_rs() -> String {
    r#"use axum::routing::{get, post, put};
use axum::{middleware, Router};

use super::handler;
use crate::auth::require_auth;
use crate::state::AppState;

/// Routes for the `users` resource. `register`/`login` and reads are public; writes require
/// a valid bearer token.
pub fn routes() -> Router<AppState> {
    let public = Router::new()
        .route("/users/register", post(handler::register))
        .route("/users/login", post(handler::login))
        .route("/users", get(handler::list))
        .route("/users/:id", get(handler::show));

    let protected = Router::new()
        .route("/users", post(handler::create))
        .route("/users/:id", put(handler::update).delete(handler::delete))
        .route_layer(middleware::from_fn(require_auth));

    public.merge(protected)
}
"#
    .to_string()
}

/// `CREATE TABLE users` migration. Adds a `UNIQUE` constraint on `email` and defaults
/// `is_admin` to `false` — both natural for a users table and not covered by the generic
/// model-driven migration (which keeps every column plain `NOT NULL`). Dialect-aware (ADR-015):
/// the primary key, `is_admin` type/default and timestamps follow `dialect`.
pub fn migration_sql(dialect: Dialect) -> String {
    let bool_ty = dialect.column_type(FieldType::Bool);
    // Postgres has a `false` literal; SQLite stores booleans as `0`/`1`.
    let false_default = match dialect {
        Dialect::Postgres => "false",
        Dialect::Sqlite => "0",
    };
    let ts = dialect.timestamp_type_default();
    format!(
        r#"-- Migration: create users
CREATE TABLE users (
    {id_pk},
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL,
    is_admin {bool_ty} NOT NULL DEFAULT {false_default},
    created_at {ts},
    updated_at {ts}
);
"#,
        id_pk = dialect.id_pk_ddl(),
    )
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
        let out = migration_sql(Dialect::Postgres);
        assert!(out.contains("CREATE TABLE users"));
        assert!(out.contains("email TEXT NOT NULL UNIQUE"));
        assert!(out.contains("is_admin BOOLEAN NOT NULL DEFAULT false"));
    }
}
