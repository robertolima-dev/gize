//! Authentication: Argon2id password hashing + stateless JWT (HS256). See gize ADR-013.
//!
//! The signing secret comes from the `GIZE_JWT_SECRET` environment variable. Tokens are
//! stateless (no server-side session), so they are valid until they expire.

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use axum::extract::Request;
use axum::http::{StatusCode, header};
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
    /// `GIZE_JWT_SECRET` is not set — a server misconfiguration, not the client's fault.
    MissingSecret,
    /// An internal hashing/signing failure.
    Internal,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            AuthError::MissingSecret => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "authentication is misconfigured",
            ),
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

/// JWT claims: the subject (user id), issued-at and expiry (seconds since the Unix epoch).
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
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

/// Issue a signed JWT for a user id.
pub fn issue_token(user_id: &uuid::Uuid) -> Result<String, AuthError> {
    let now = now_secs();
    let claims = Claims {
        sub: user_id.to_string(),
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
    let token = value
        .strip_prefix("Bearer ")
        .ok_or(AuthError::Unauthorized)?;
    verify_token(token)
}

/// Axum middleware that rejects unauthenticated requests. Apply it with
/// `.route_layer(axum::middleware::from_fn(require_auth))` on the routes you want to protect.
pub async fn require_auth(req: Request, next: Next) -> Result<Response, AuthError> {
    bearer_claims(&req)?;
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
        let token = issue_token(&id).unwrap();
        let claims = verify_token(&token).unwrap();
        assert_eq!(claims.sub, id.to_string());
        assert!(verify_token("not-a-token").is_err());
    }
}
