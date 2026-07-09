//! Template for the generated `src/auth/mod.rs`: Argon2 password hashing plus stateless
//! JWT (HS256) auth, a `require_auth` middleware that guards mutating routes (ADR-013), and a
//! `require_admin` role guard for admin-only routes (ADR-021).
//!
//! This is plain, owned code emitted into the project — no hidden framework. The signing
//! secret is read from `GIZE_JWT_SECRET` in the environment, never from `gize.toml`.

/// `src/auth/mod.rs` — the whole auth module for a generated project.
pub fn mod_rs() -> String {
    r#"//! Authentication: Argon2id password hashing + stateless JWT (HS256). See gize ADR-013/ADR-021.
//!
//! The signing secret comes from the `GIZE_JWT_SECRET` environment variable. Tokens are
//! stateless (no server-side session), so they are valid until they expire. The token also
//! carries an `is_admin` claim, which `require_admin` uses to gate admin-only routes.

// This module is a provided toolkit: depending on which resources you generate, some helpers
// may not be called yet (a project with no `users` resource issues no tokens and hashes no
// passwords; one with no generic CRUD never calls `require_auth`). Allowed to have unused items
// so any project shape stays clippy-clean under `-D warnings`; delete what you do not need.
#![allow(dead_code)]

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::Request;
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// How long an issued token stays valid. Kept short since stateless JWTs cannot be revoked.
const TOKEN_TTL_SECS: u64 = 24 * 60 * 60;

/// Authentication failures, mapped to HTTP status codes.
#[derive(Debug)]
pub enum AuthError {
    /// Missing, malformed, or expired credentials.
    Unauthorized,
    /// Authenticated but not allowed — e.g. a non-admin calling an admin-only route.
    Forbidden,
    /// `GIZE_JWT_SECRET` is not set — a server misconfiguration, not the client's fault.
    MissingSecret,
    /// An internal hashing/signing failure.
    Internal,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AuthError::Forbidden => (StatusCode::FORBIDDEN, "forbidden"),
            AuthError::MissingSecret => {
                (StatusCode::INTERNAL_SERVER_ERROR, "authentication is misconfigured")
            }
            AuthError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "authentication error"),
        };
        (status, message).into_response()
    }
}

/// Hash a plaintext password with Argon2id for storage.
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AuthError::Internal)
}

/// Verify a plaintext password against a stored Argon2 hash (constant-time within Argon2).
pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

/// JWT claims: the subject (user id), the admin flag, issued-at and expiry (seconds since the
/// Unix epoch). `is_admin` is captured at login so `require_admin` needs no per-request DB read.
///
/// `require_auth` stores the validated `Claims` in the request extensions, so a guarded handler
/// can read the caller's identity with `Extension<Claims>` (see the `users` `me` route).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub is_admin: bool,
    pub iat: u64,
    pub exp: u64,
}

fn secret() -> Result<String, AuthError> {
    std::env::var("GIZE_JWT_SECRET").map_err(|_| AuthError::MissingSecret)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Issue a signed JWT for a user id, embedding whether the user is an admin.
pub fn issue_token(user_id: &uuid::Uuid, is_admin: bool) -> Result<String, AuthError> {
    let now = now_secs();
    let claims = Claims {
        sub: user_id.to_string(),
        is_admin,
        iat: now,
        exp: now + TOKEN_TTL_SECS,
    };
    let key = jsonwebtoken::EncodingKey::from_secret(secret()?.as_bytes());
    jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &key)
        .map_err(|_| AuthError::Internal)
}

/// Validate a bearer token, returning its claims (expiry is checked automatically).
pub fn verify_token(token: &str) -> Result<Claims, AuthError> {
    let key = jsonwebtoken::DecodingKey::from_secret(secret()?.as_bytes());
    jsonwebtoken::decode::<Claims>(token, &key, &jsonwebtoken::Validation::default())
        .map(|data| data.claims)
        .map_err(|_| AuthError::Unauthorized)
}

/// Extract and validate the `Authorization: Bearer <jwt>` header.
fn bearer_claims(req: &Request) -> Result<Claims, AuthError> {
    let value = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::Unauthorized)?;
    let token = value.strip_prefix("Bearer ").ok_or(AuthError::Unauthorized)?;
    verify_token(token)
}

/// Axum middleware that rejects unauthenticated requests. Apply it with
/// `.route_layer(axum::middleware::from_fn(require_auth))` on the routes you want to protect.
/// The default `users` slice uses `require_admin`; generic `make crud` resources use this.
///
/// The validated `Claims` are inserted into the request extensions, so a guarded handler can
/// recover the caller's identity with `Extension<Claims>` (e.g. the `users` `me` self-service
/// route).
pub async fn require_auth(mut req: Request, next: Next) -> Result<Response, AuthError> {
    let claims = bearer_claims(&req)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

/// Axum middleware that requires the caller to be an admin. Rejects with `401` when the token
/// is missing/invalid/expired and `403` when the caller is authenticated but not an admin.
/// Apply it with `.route_layer(axum::middleware::from_fn(require_admin))`.
///
/// The admin flag comes from the token (set at login), so a token stays admin until it expires
/// even if the account's `is_admin` later changes — the usual stateless-JWT trade-off, kept
/// small by the short `TOKEN_TTL_SECS`.
pub async fn require_admin(req: Request, next: Next) -> Result<Response, AuthError> {
    let claims = bearer_claims(&req)?;
    if !claims.is_admin {
        return Err(AuthError::Forbidden);
    }
    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_hash_roundtrip() {
        let hash = hash_password("s3cr3t").unwrap();
        assert!(verify_password("s3cr3t", &hash));
        assert!(!verify_password("wrong", &hash));
    }

    #[test]
    fn token_roundtrip() {
        // SAFETY: test-only; sets the secret for this process.
        unsafe { std::env::set_var("GIZE_JWT_SECRET", "test-secret") };
        let id = uuid::Uuid::new_v4();
        let token = issue_token(&id, false).unwrap();
        let claims = verify_token(&token).unwrap();
        assert_eq!(claims.sub, id.to_string());
        assert!(!claims.is_admin);
        assert!(verify_token("not-a-token").is_err());
    }

    #[test]
    fn admin_flag_travels_in_token() {
        // SAFETY: test-only; sets the secret for this process.
        unsafe { std::env::set_var("GIZE_JWT_SECRET", "test-secret") };
        let token = issue_token(&uuid::Uuid::new_v4(), true).unwrap();
        assert!(verify_token(&token).unwrap().is_admin);
    }
}
"#
    .to_string()
}
