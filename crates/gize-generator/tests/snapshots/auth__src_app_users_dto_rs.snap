use serde::{Deserialize, Serialize};
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

/// Payload for the public `POST /users/register`. It has no `is_admin`: a public sign-up never
/// creates an admin. A client that still sends the field is accepted and the field is ignored.
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 1, message = "must not be empty"))]
    pub name: String,
    #[validate(email(message = "must be a valid email"))]
    pub email: String,
    #[validate(length(min = 8, message = "must be at least 8 characters"))]
    pub password: String,
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

/// Default and maximum page sizes for `GET` list routes (ADR-022).
pub const DEFAULT_PER_PAGE: u32 = 25;
pub const MAX_PER_PAGE: u32 = 100;

/// Query parameters of a list route: `?page=<1-based>&per_page=<1..=100>`.
#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl ListParams {
    /// Apply the defaults and reject out-of-range values. Returns `(page, per_page)`.
    pub fn resolve(&self) -> Result<(u32, u32), String> {
        let page = self.page.unwrap_or(1);
        let per_page = self.per_page.unwrap_or(DEFAULT_PER_PAGE);
        if page == 0 {
            return Err("page must be at least 1".to_string());
        }
        if per_page == 0 || per_page > MAX_PER_PAGE {
            return Err(format!("per_page must be between 1 and {MAX_PER_PAGE}"));
        }
        Ok((page, per_page))
    }
}

/// One page of a list route. `total` counts every row, not only this page.
#[derive(Debug, Serialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}
